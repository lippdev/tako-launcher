// Adaptado de lippdev/consolemode: src/ConsoleMode/ConfirmWindow.xaml.cs
using System.Windows.Threading;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// "Você consegue ver esta tela?" na tela de jogo, como o "Manter estas configurações de vídeo?" do Windows. Responde
/// falso se o tempo acabar, se escolherem "Voltar ao normal" ou com B/Esc: quem não vê a tela não consegue confirmar.
/// </summary>
public sealed partial class ScreenConfirmViewModel : ObservableObject, IOverlayViewModel, IDisposable
{
    private readonly TaskCompletionSource<bool> _result = new(TaskCreationOptions.RunContinuationsAsynchronously);
    private readonly DispatcherTimer _timer = new() { Interval = TimeSpan.FromSeconds(1) };

    [ObservableProperty]
    [NotifyPropertyChangedFor(nameof(CountdownText))]
    private int _secondsLeft;

    public ScreenConfirmViewModel(string screenName, int seconds)
    {
        ScreenName = screenName;
        Seconds = seconds;
        _secondsLeft = seconds;
        Actions =
        [
            new OverlayAction("Sim, manter", ActionStyle.Primary, new RelayCommand(() => Finish(true)), IsDefault: true),
            new OverlayAction("Voltar ao normal", ActionStyle.Secondary, new RelayCommand(() => Finish(false))),
        ];
        _timer.Tick += (_, _) =>
        {
            SecondsLeft--;
            if (SecondsLeft <= 0)
            {
                Finish(false);
            }
        };
        _timer.Start();
    }

    public string ScreenName { get; }

    public int Seconds { get; }

    public string Title { get; } = "Você consegue ver esta tela?";

    public string Text => $"Esta é a tela de jogo ({ScreenName}). Confirme com o controle, o teclado ou o mouse. Se ninguém responder, suas telas voltam ao normal sozinhas.";

    public string CountdownText => $"Voltando ao normal em {SecondsLeft} s";

    public IReadOnlyList<OverlayAction> Actions { get; }

    public Task<bool> Result => _result.Task;

    public bool HandleBack()
    {
        Finish(false);
        return true;
    }

    public void Dispose()
    {
        _timer.Stop();
        _result.TrySetResult(false);
    }

    private void Finish(bool keep)
    {
        _timer.Stop();
        _result.TrySetResult(keep);
    }
}
