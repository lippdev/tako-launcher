using Launcher.Core.Diagnostics;
using Microsoft.Extensions.Logging;
using Xunit;

namespace Launcher.Core.Tests;

public sealed partial class DiagnosticsTests : IDisposable
{
    private readonly string _root = Path.Combine(Path.GetTempPath(), "tako-tests-" + Guid.NewGuid().ToString("N"));

    public void Dispose()
    {
        if (Directory.Exists(_root))
        {
            Directory.Delete(_root, recursive: true);
        }
    }

    [Theory]
    [InlineData(@"Falha em C:\Users\Fulano\AppData\Local\Tako\tako.db", @"Falha em %USERPROFILE%\AppData\Local\Tako\tako.db")]
    [InlineData(@"c:\users\fulano\Jogos\x.exe", @"%USERPROFILE%\Jogos\x.exe")]
    [InlineData("file:///C:/Users/Fulano/capa.png", "file:///%USERPROFILE%/capa.png")]
    [InlineData(@"D:\Jogos\x.exe", @"D:\Jogos\x.exe")]
    public void Redactor_replaces_the_user_profile(string input, string expected) =>
        Assert.Equal(expected, new LogRedactor(@"C:\Users\Fulano").Redact(input));

    [Theory]
    [InlineData("GET https://api.exemplo.com/v1/jogos?access_token=abc123&x=1 falhou", "GET https://api.exemplo.com/v1/jogos?[removido] falhou")]
    [InlineData("baixando https://cdn.exemplo.com/a.zip?sig=XYZ", "baixando https://cdn.exemplo.com/a.zip?[removido]")]
    [InlineData("Authorization: Bearer eyJhbGciOi.payload.sig", "Authorization: [removido] [removido]")]
    [InlineData("token=segredo; apiKey: 123", "token=[removido]; apiKey: [removido]")]
    [InlineData("https://store.steampowered.com/app/570", "https://store.steampowered.com/app/570")]
    public void Redactor_removes_tokens_and_authenticated_queries(string input, string expected) =>
        Assert.Equal(expected, new LogRedactor(null).Redact(input));

    [Fact]
    public void File_logger_writes_information_and_above_redacted()
    {
        var time = new FixedTimeProvider(new DateTimeOffset(2026, 10, 15, 12, 0, 0, TimeSpan.Zero));
        using (var provider = new FileLoggerProvider(_root, time, new LogRedactor(@"C:\Users\Fulano")))
        {
            var logger = provider.CreateLogger("Tako.Teste");
            Assert.False(logger.IsEnabled(LogLevel.Debug));
            LogHidden(logger);
            LogOpening(logger, @"C:\Users\Fulano\jogo.exe");
            LogFailure(logger, new InvalidOperationException("quebrou"), "https://x.com/a?token=1");
            Assert.True(provider.Flush(TimeSpan.FromSeconds(5)));
        }

        string file = Path.Combine(_root, FileLoggerProvider.FileName(time.GetLocalNow().DateTime));
        string text = File.ReadAllText(file);
        Assert.DoesNotContain("não aparece", text, StringComparison.Ordinal);
        Assert.Contains(@"[INF] Tako.Teste: abrindo %USERPROFILE%\jogo.exe", text, StringComparison.Ordinal);
        Assert.Contains("[ERR] Tako.Teste: erro com https://x.com/a?[removido]", text, StringComparison.Ordinal);
        Assert.Contains("InvalidOperationException: quebrou", text, StringComparison.Ordinal);
        Assert.DoesNotContain("Fulano", text, StringComparison.Ordinal);
    }

    [Fact]
    public async Task File_logger_is_safe_from_many_threads()
    {
        var time = new FixedTimeProvider(new DateTimeOffset(2026, 10, 15, 12, 0, 0, TimeSpan.Zero));
        using (var provider = new FileLoggerProvider(_root, time, new LogRedactor(null)))
        {
            var logger = provider.CreateLogger("Paralelo");
            await Task.WhenAll(Enumerable.Range(0, 8).Select(t => Task.Run(() =>
            {
                for (int i = 0; i < 100; i++)
                {
                    LogLine(logger, t, i);
                }
            })));
        }

        string file = Path.Combine(_root, FileLoggerProvider.FileName(time.GetLocalNow().DateTime));
        Assert.Equal(800, File.ReadAllLines(file).Length);
    }

    [Fact]
    public void Expired_logs_are_deleted_after_seven_days()
    {
        Directory.CreateDirectory(_root);
        var today = new DateTime(2026, 10, 15);
        foreach (int daysAgo in new[] { 0, 6, 7, 30 })
        {
            File.WriteAllText(Path.Combine(_root, FileLoggerProvider.FileName(today.AddDays(-daysAgo))), "x");
        }

        File.WriteAllText(Path.Combine(_root, "outro.log"), "x");

        FileLoggerProvider.DeleteExpired(_root, today);

        Assert.Equal(
            ["outro.log", "tako-20261009.log", "tako-20261015.log"],
            Directory.GetFiles(_root).Select(Path.GetFileName).Order(StringComparer.Ordinal));
    }

    [LoggerMessage(Level = LogLevel.Debug, Message = "não aparece")]
    private static partial void LogHidden(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "abrindo {Path}")]
    private static partial void LogOpening(ILogger logger, string path);

    [LoggerMessage(Level = LogLevel.Error, Message = "erro com {Url}")]
    private static partial void LogFailure(ILogger logger, Exception exception, string url);

    [LoggerMessage(Level = LogLevel.Information, Message = "linha {Thread}-{Index}")]
    private static partial void LogLine(ILogger logger, int thread, int index);
}
