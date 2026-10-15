using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class RtssLocatorTests
{
    [Fact]
    public void The_registry_folder_comes_first_then_both_program_files_without_repeats()
    {
        string x86 = Path.Combine(Path.GetTempPath(), "Program Files (x86)");
        string x64 = Path.Combine(Path.GetTempPath(), "Program Files");
        string registry = Path.Combine(x86, "RivaTuner Statistics Server");
        var folders = RtssLocator.Folders($"\"{registry}{Path.DirectorySeparatorChar}\"", x86, x64);
        Assert.Equal([registry, Path.Combine(x64, "RivaTuner Statistics Server")], folders);
        Assert.Empty(RtssLocator.Folders(" ", null, null));
    }

    [Fact]
    public void Resolve_returns_the_folder_that_has_rtss()
    {
        string[] folders = [Path.Combine(Path.GetTempPath(), "a"), Path.Combine(Path.GetTempPath(), "b")];
        string exe = Path.Combine(folders[1], RtssLocator.ExeName);
        Assert.Equal(folders[1], RtssLocator.Resolve(folders, p => p == exe));
        Assert.Null(RtssLocator.Resolve(folders, _ => false));
    }
}
