using System.Diagnostics;
using System.IO;
using System.Text.Json;

namespace Launcher.Bench;

/// <summary>
/// "Jogo" de carga: ocupa todos os núcleos com contas de ponto flutuante e mede quanto trabalho fez por segundo. Rodado
/// sozinho dá a linha de base; aberto pelo Tako (como jogo adicionado) mostra quanto o launcher tira do jogo.
/// </summary>
internal static class LoadWorkload
{
    public static int Run(Options options)
    {
        int seconds = options.Int("seconds", 10);
        int warmup = options.Int("warmup", 2);
        string result = options.Require("result");
        int threads = Environment.ProcessorCount;

        // Um contador por linha de cache (16 longs = 128 bytes) para as threads não disputarem a mesma linha.
        long[] counters = new long[threads * 16];
        bool stop = false;
        var workers = Enumerable.Range(0, threads).Select(i => new Thread(() =>
        {
            double x = 1.000001 + i;
            while (!Volatile.Read(ref stop))
            {
                for (int k = 0; k < 20_000; k++)
                {
                    x = Math.Sqrt((x * 1.0000001) + k) + 1;
                }

                Volatile.Write(ref counters[i * 16], counters[i * 16] + 1);
            }

            GC.KeepAlive(x);
        })
        { IsBackground = true, Name = $"carga {i}" }).ToList();

        var clock = Stopwatch.StartNew();
        workers.ForEach(t => t.Start());
        Thread.Sleep(TimeSpan.FromSeconds(warmup));
        long start = Sum(counters, threads);
        double startSeconds = clock.Elapsed.TotalSeconds;
        Thread.Sleep(TimeSpan.FromSeconds(seconds));
        long end = Sum(counters, threads);
        double elapsed = clock.Elapsed.TotalSeconds - startSeconds;
        Volatile.Write(ref stop, true);
        workers.ForEach(t => t.Join());

        var outcome = new LoadResult(threads, Math.Round(elapsed, 3), Math.Round((end - start) / elapsed, 1));
        Directory.CreateDirectory(Path.GetDirectoryName(Path.GetFullPath(result)) ?? ".");
        File.WriteAllText(result, JsonSerializer.Serialize(outcome, Json.Options));
        return 0;
    }

    private static long Sum(long[] counters, int threads)
    {
        long total = 0;
        for (int i = 0; i < threads; i++)
        {
            total += Volatile.Read(ref counters[i * 16]);
        }

        return total;
    }
}

/// <summary>Resultado da carga: blocos de trabalho por segundo em todas as threads.</summary>
internal sealed record LoadResult(int Threads, double Seconds, double OpsPerSecond);
