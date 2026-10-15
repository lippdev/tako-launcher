using System.Globalization;
using System.Text;
using System.Threading.Channels;
using Microsoft.Extensions.Logging;

namespace Launcher.Core.Diagnostics;

/// <summary>
/// Logs de diagnóstico em arquivo (MVP-033): <c>tako-AAAAMMDD.log</c> em <paramref name="directory"/>, um por dia,
/// guardando <see cref="RetentionDays"/> dias. Quem registra só enfileira (não bloqueia a UI); uma tarefa em segundo
/// plano grava. Nível mínimo Information; o texto passa pelo <see cref="LogRedactor"/>.
/// </summary>
public sealed class FileLoggerProvider : ILoggerProvider
{
    public const int RetentionDays = 7;
    private const string Prefix = "tako-";
    private const string Extension = ".log";

    private readonly string _directory;
    private readonly TimeProvider _time;
    private readonly LogRedactor _redactor;
    private readonly Channel<string> _queue;
    private readonly Task _writer;
    private int _pending;

    public FileLoggerProvider(string directory, TimeProvider time, LogRedactor? redactor = null)
    {
        _directory = directory;
        _time = time;
        _redactor = redactor ?? LogRedactor.Default;
        // Rajada maior que a fila: descarta as mais antigas em vez de travar quem registra.
        _queue = Channel.CreateBounded<string>(
            new BoundedChannelOptions(10_000) { FullMode = BoundedChannelFullMode.DropOldest, SingleReader = true },
            _ => Interlocked.Decrement(ref _pending));
        _writer = Task.Run(WriteLoopAsync);
    }

    public string Directory => _directory;

    public static string FileName(DateTime localDate) => Prefix + localDate.ToString("yyyyMMdd", CultureInfo.InvariantCulture) + Extension;

    public ILogger CreateLogger(string categoryName) => new FileLogger(this, categoryName);

    /// <summary>Espera a fila esvaziar (ex.: antes de o processo cair por uma exceção não tratada).</summary>
    public bool Flush(TimeSpan timeout) => SpinWait.SpinUntil(() => Volatile.Read(ref _pending) == 0, timeout);

    public void Dispose()
    {
        _queue.Writer.TryComplete();
        _writer.Wait(TimeSpan.FromSeconds(3));
    }

    /// <summary>Apaga os <c>tako-*.log</c> com mais de <see cref="RetentionDays"/> dias (o de hoje conta como um).</summary>
    public static void DeleteExpired(string directory, DateTime today)
    {
        if (!System.IO.Directory.Exists(directory))
        {
            return;
        }

        var oldest = today.Date.AddDays(-(RetentionDays - 1));
        foreach (string file in System.IO.Directory.EnumerateFiles(directory, Prefix + "*" + Extension))
        {
            string stamp = Path.GetFileNameWithoutExtension(file)[Prefix.Length..];
            if (DateTime.TryParseExact(stamp, "yyyyMMdd", CultureInfo.InvariantCulture, DateTimeStyles.None, out var date) && date < oldest)
            {
                try
                {
                    File.Delete(file);
                }
                catch (IOException)
                {
                    // Aberto por outro programa: tenta de novo no próximo dia.
                }
                catch (UnauthorizedAccessException)
                {
                }
            }
        }
    }

    private void Enqueue(string line)
    {
        Interlocked.Increment(ref _pending);
        if (!_queue.Writer.TryWrite(line))
        {
            Interlocked.Decrement(ref _pending);
        }
    }

    private string Format(LogLevel level, string category, string message, Exception? exception)
    {
        var now = _time.GetLocalNow();
        var line = new StringBuilder()
            .Append(now.ToString("yyyy-MM-dd HH:mm:ss.fff zzz", CultureInfo.InvariantCulture))
            .Append(" [").Append(LevelLabel(level)).Append("] ")
            .Append(category).Append(": ").Append(message);
        if (exception is not null)
        {
            line.AppendLine().Append(exception);
        }

        return _redactor.Redact(line.ToString());
    }

    private async Task WriteLoopAsync()
    {
        StreamWriter? writer = null;
        DateTime day = default;
        try
        {
            while (await _queue.Reader.WaitToReadAsync().ConfigureAwait(false))
            {
                var today = _time.GetLocalNow().Date;
                if (writer is null || today != day)
                {
                    writer?.Dispose();
                    writer = null;
                    day = today;
                    try
                    {
                        System.IO.Directory.CreateDirectory(_directory);
                        DeleteExpired(_directory, today);
                        var stream = new FileStream(Path.Combine(_directory, FileName(today)), FileMode.Append, FileAccess.Write, FileShare.ReadWrite | FileShare.Delete);
                        writer = new StreamWriter(stream, new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
                    }
                    catch (Exception ex) when (ex is IOException or UnauthorizedAccessException)
                    {
                        // Sem onde gravar (disco cheio, pasta protegida): descarta, mas nunca derruba o launcher.
                        writer = null;
                    }
                }

                while (_queue.Reader.TryRead(out string? line))
                {
                    try
                    {
                        writer?.WriteLine(line);
                    }
                    catch (IOException)
                    {
                    }
                    finally
                    {
                        Interlocked.Decrement(ref _pending);
                    }
                }

                try
                {
                    writer?.Flush();
                }
                catch (IOException)
                {
                }
            }
        }
        finally
        {
            writer?.Dispose();
        }
    }

    private static string LevelLabel(LogLevel level) => level switch
    {
        LogLevel.Information => "INF",
        LogLevel.Warning => "WRN",
        LogLevel.Error => "ERR",
        LogLevel.Critical => "CRT",
        LogLevel.Debug => "DBG",
        _ => "TRC",
    };

    private sealed class FileLogger(FileLoggerProvider provider, string category) : ILogger
    {
        public IDisposable? BeginScope<TState>(TState state)
            where TState : notnull => null;

        public bool IsEnabled(LogLevel logLevel) => logLevel is >= LogLevel.Information and < LogLevel.None;

        public void Log<TState>(LogLevel logLevel, EventId eventId, TState state, Exception? exception, Func<TState, Exception?, string> formatter)
        {
            if (!IsEnabled(logLevel))
            {
                return;
            }

            ArgumentNullException.ThrowIfNull(formatter);
            provider.Enqueue(provider.Format(logLevel, category, formatter(state, exception), exception));
        }
    }
}
