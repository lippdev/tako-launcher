using System.Globalization;

namespace Launcher.Bench;

/// <summary>
/// <c>Launcher.Bench.exe [run]</c> (padrão): roda o benchmark completo e grava <c>benchmark.json</c> e <c>benchmark.md</c>.
/// Modos internos (o próprio benchmark chama): <c>child</c> abre o Tako num processo novo e mede;
/// <c>load</c> é o "jogo" de carga de CPU.
/// </summary>
internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        bool explicitMode = args.Length > 0 && !args[0].StartsWith("--", StringComparison.Ordinal);
        string mode = explicitMode ? args[0] : "run";
        try
        {
            var options = Options.Parse(explicitMode ? args.Skip(1) : args);
            return mode switch
            {
                "run" => Orchestrator.Run(options),
                "child" => ChildRunner.Run(options),
                "load" => LoadWorkload.Run(options),
                _ => Usage($"Modo desconhecido: {mode}"),
            };
        }
        catch (Exception ex) when (ex is ArgumentException or FormatException)
        {
            return Usage(ex.Message);
        }
    }

    private static int Usage(string problem)
    {
        Console.Error.WriteLine(problem);
        Console.Error.WriteLine("""
            Uso: Launcher.Bench.exe [--sizes 20,200,1000] [--warm 5] [--idle-seconds 10] [--load-seconds 10]
                                    [--out pasta] [--label "nome da máquina"] [--quick]
            --quick: rodada curta para conferir que tudo funciona (só 20 jogos, poucos segundos).
            """);
        return 2;
    }
}

/// <summary>Opções <c>--nome valor</c> (e <c>--quick</c>, sem valor).</summary>
internal sealed class Options
{
    private readonly Dictionary<string, string> _values = new(StringComparer.OrdinalIgnoreCase);

    public static Options Parse(IEnumerable<string> args)
    {
        var options = new Options();
        string? key = null;
        foreach (string arg in args)
        {
            if (arg.StartsWith("--", StringComparison.Ordinal))
            {
                if (key is not null)
                {
                    options._values[key] = "true";
                }

                key = arg[2..];
            }
            else if (key is not null)
            {
                options._values[key] = arg;
                key = null;
            }
            else
            {
                throw new ArgumentException($"Valor sem opção: {arg}");
            }
        }

        if (key is not null)
        {
            options._values[key] = "true";
        }

        return options;
    }

    public bool Has(string key) => _values.ContainsKey(key);

    public string? Text(string key) => _values.GetValueOrDefault(key);

    public string Require(string key) => Text(key) ?? throw new ArgumentException($"Falta --{key}");

    public int Int(string key, int fallback) =>
        Text(key) is { } value ? int.Parse(value, NumberStyles.Integer, CultureInfo.InvariantCulture) : fallback;

    public IReadOnlyList<int> Ints(string key, IReadOnlyList<int> fallback) =>
        Text(key) is { } value
            ? [.. value.Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries).Select(v => int.Parse(v, NumberStyles.Integer, CultureInfo.InvariantCulture))]
            : fallback;
}
