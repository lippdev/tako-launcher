// Adaptado de lippdev/consolemode: src/ConsoleMode/Services/AltTabRules.cs
namespace ConsoleMode.Runtime.SessionTools;

/// <summary>O que o alternador de janelas precisa saber de uma janela de topo para decidir se ela aparece.</summary>
public readonly record struct WindowFacts(
    string Title,
    string ClassName,
    bool Visible,
    bool Cloaked,
    bool HasOwner,
    bool IsToolWindow,
    bool IsAppWindow,
    bool IsOwnProcess);

/// <summary>
/// Regras do alternador de janelas da Central de Controle (o "Alt+Tab" do controle), as mesmas que o Windows usa no
/// dele: visível, não oculta pelo DWM (app UWP suspenso), com título e sem ser janela de ferramenta nem diálogo com
/// dono, a não ser que ela se declare janela de app. Separadas para serem testadas.
/// </summary>
public static class AltTabRules
{
    private static readonly HashSet<string> ShellClasses = new(StringComparer.Ordinal)
    {
        "Progman", "WorkerW", "Shell_TrayWnd", "Shell_SecondaryTrayWnd", "Windows.UI.Core.CoreWindow",
    };

    public static bool ShouldList(WindowFacts window)
    {
        if (!window.Visible || window.Cloaked || window.IsOwnProcess)
        {
            return false;
        }

        if (string.IsNullOrWhiteSpace(window.Title) || ShellClasses.Contains(window.ClassName))
        {
            return false;
        }

        return window.IsAppWindow || (!window.IsToolWindow && !window.HasOwner);
    }

    /// <summary>"chrome.exe" → "Chrome"; vazio quando não há nome.</summary>
    public static string ProcessDisplayName(string? processName)
    {
        if (string.IsNullOrWhiteSpace(processName))
        {
            return string.Empty;
        }

        string name = processName.Trim();
        if (name.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
        {
            name = name[..^4];
        }

        return name.Length == 0 ? string.Empty : char.ToUpperInvariant(name[0]) + name[1..];
    }
}
