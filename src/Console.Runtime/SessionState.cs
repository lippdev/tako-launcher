namespace Console.Runtime;

/// <summary>Estados de sessão (PRD §4.4). Transições devem ser idempotentes.</summary>
public enum SessionState
{
    Idle,
    EnteringFullscreen,
    Browsing,
    LaunchingGame,
    Playing,
    Returning,
    Restoring,
    Error,
    Recovery,
    DesktopFallback,
}
