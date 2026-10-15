namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Uma janela aberta no alternador da Central de Controle.</summary>
public sealed record OpenWindow(nint Handle, string Title, string ProcessName, string? ExePath);

/// <summary>
/// Alternador de janelas da Central de Controle (o "Alt+Tab" do controle): lista as janelas que o Alt+Tab listaria,
/// da frente para trás, traz uma para a frente e pede para outra fechar. Nunca lança: janela que some no meio é pulada.
/// </summary>
public interface IWindowSwitcher
{
    IReadOnlyList<OpenWindow> List();

    /// <summary>Ícone do programa da janela no tamanho pedido; nulo mantém o ícone genérico.</summary>
    IconBitmap? LoadIcon(OpenWindow window, int size);

    /// <summary>Traz a janela para a frente (restaura se minimizada), mesmo com outro programa em primeiro plano.</summary>
    bool Activate(nint handle);

    /// <summary>Pede para fechar, como o X da janela: o programa ainda pode perguntar se quer salvar.</summary>
    bool RequestClose(nint handle);
}

/// <summary>
/// Contador e limite de FPS pelo RivaTuner Statistics Server (RTSS), quando instalado. Sem RTSS, tudo devolve falso e
/// nada muda no sistema. O limite original do usuário é guardado na primeira mudança e volta em <see cref="Restore"/>.
/// </summary>
public interface IFpsOverlayService
{
    bool IsInstalled { get; }

    /// <summary>O limite de FPS pode ser mudado (biblioteca de perfis do RTSS encontrada).</summary>
    bool CanLimitFps { get; }

    /// <summary>Mostra (ou tira) o contador do Tako no estilo pedido; inicia o RTSS se preciso.</summary>
    bool ApplyOverlay(string style, FpsOverlayLayout layout);

    /// <summary>Limite global de FPS (0 = sem limite).</summary>
    bool SetFpsLimit(int fps);

    /// <summary>Fim da sessão: tira o contador do Tako e devolve o limite que o usuário tinha.</summary>
    void Restore();
}

/// <summary>Ponteiro do mouse durante a sessão: esconder e mover com o analógico.</summary>
public interface IPointerService
{
    bool IsCursorHidden { get; }

    bool IsControllerMouseOn { get; }

    /// <summary>Com a Central de Controle aberta o controle navega o menu, não o ponteiro.</summary>
    bool IsControllerMousePaused { get; set; }

    void SetCursorHidden(bool hidden);

    /// <param name="screenHeight">Altura da tela do jogo, para a velocidade ser a mesma em qualquer resolução.</param>
    void SetControllerMouse(bool enabled, int screenHeight);

    /// <summary>Fim da sessão ou do app: ponteiro visível e analógico de volta ao jogo.</summary>
    void Reset();

    /// <summary>Na abertura: devolve os cursores se uma execução anterior caiu com o ponteiro escondido.</summary>
    void RecoverAfterCrash();
}

/// <summary>Abre o ControlFS, explorador de arquivos para controle instalado à parte. Nunca o encerra.</summary>
public interface IControlFsService
{
    bool IsInstalled { get; }

    /// <summary>Abre (ou traz para a frente). Falso quando não está instalado ou não abriu.</summary>
    bool Open();
}

/// <summary>Leitura de CPU, GPU e memória pelos contadores do próprio Windows; uma chamada por segundo basta.</summary>
public interface IHardwareMonitor
{
    HardwareSample Sample();
}
