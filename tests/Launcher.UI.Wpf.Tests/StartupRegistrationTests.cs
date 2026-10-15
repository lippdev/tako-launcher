using Launcher.UI.Wpf.Services;
using Microsoft.Win32;
using Xunit;

namespace Launcher.UI.Wpf.Tests;

/// <summary>"Abrir ao iniciar o Windows" num ramo de teste do HKCU (nunca no Run real da máquina do CI).</summary>
public sealed class StartupRegistrationTests : IDisposable
{
    private const string Exe = @"C:\Users\Ana\AppData\Local\Programs\Tako\Tako.exe";
    private readonly string _root = $@"Software\Tako-Tests\{Guid.NewGuid():N}";

    private string RunKey => _root + @"\Run";

    public void Dispose() => Registry.CurrentUser.DeleteSubKeyTree(_root, throwOnMissingSubKey: false);

    [Fact]
    public void Enabling_writes_the_quoted_executable_and_disabling_removes_it()
    {
        var startup = new StartupRegistration(RunKey);
        Assert.False(startup.IsEnabled(Exe));

        startup.Set(true, Exe);
        using (var key = Registry.CurrentUser.OpenSubKey(RunKey))
        {
            Assert.Equal($"\"{Exe}\"", key?.GetValue(StartupRegistration.ValueName));
        }

        Assert.True(startup.IsEnabled(Exe));

        startup.Set(false, Exe);
        Assert.False(startup.IsEnabled(Exe));
        startup.Set(false, Exe); // remover de novo não falha
    }

    [Fact]
    public void Entry_for_another_copy_reads_as_disabled()
    {
        using (var key = Registry.CurrentUser.CreateSubKey(RunKey))
        {
            key.SetValue(StartupRegistration.ValueName, @"""D:\Portatil\Tako.exe""");
        }

        Assert.False(new StartupRegistration(RunKey).IsEnabled(Exe));
    }
}
