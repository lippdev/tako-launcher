namespace ConsoleMode.Runtime.Display;

/// <summary>
/// Atalho de teclado global de emergência (MVP-036): devolve as telas para a mesa mesmo com o jogo em primeiro plano ou
/// com a interface ocupada. A ação roda fora da thread da UI.
/// </summary>
public interface IEmergencyHotkey : IDisposable
{
    /// <summary>Teclas, para mostrar ao usuário (ex.: <c>Ctrl+Alt+Shift+D</c>).</summary>
    string Keys { get; }

    /// <summary>Registra o atalho. Falso se outro programa já usa a combinação (o Tako segue sem o atalho).</summary>
    bool Start(Action pressed);
}
