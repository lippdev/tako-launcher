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

/// <summary>Estado do contador de FPS durante a sessão.</summary>
public enum FpsCounterStatus
{
    /// <summary>Parado (fora de jogo ou contador desligado).</summary>
    Idle,

    /// <summary>PresentMon aberto, esperando o primeiro quadro do jogo.</summary>
    Starting,

    Running,

    /// <summary>O PresentMon não veio com esta compilação do Tako (ex.: rodando pelo código-fonte).</summary>
    Unavailable,

    /// <summary>O Windows recusou a medição: falta a permissão (ou o login depois dela).</summary>
    Denied,

    Failed,
}

/// <summary>
/// Contador de FPS sem injeção (ADR-0017): o PresentMon lê os eventos de apresentação do Windows (ETW) e o Tako filtra
/// os quadros dos processos do jogo. Nada é carregado dentro do jogo. Nunca lança.
/// </summary>
public interface IFpsCounter
{
    /// <summary>O PresentMon está ao lado do Tako.exe.</summary>
    bool IsAvailable { get; }

    FpsCounterStatus Status { get; }

    /// <summary>
    /// Começa a medir. <paramref name="gameProcesses"/> é consultado a cada segundo: o jogo pode trocar de processo.
    /// Já medindo, só troca a fonte dos processos.
    /// </summary>
    bool StartCapture(Func<IReadOnlyCollection<int>> gameProcesses);

    /// <summary>Números de agora; nulo sem quadros recentes do jogo.</summary>
    FrameStats? Current { get; }

    /// <summary>Fecha o PresentMon e a sessão ETW do Tako. Seguro chamar parado.</summary>
    void StopCapture();

    /// <summary>Na abertura: fecha uma sessão ETW do Tako que uma execução que caiu deixou aberta.</summary>
    void RecoverAfterCrash();
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
