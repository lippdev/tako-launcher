using System.Windows;
using System.Windows.Controls;
using System.Windows.Media;
using System.Windows.Media.Animation;
using System.Windows.Shapes;
using Launcher.UI.Wpf.ViewModels;

namespace Launcher.UI.Wpf.Controls;

/// <summary>
/// Fundo dinâmico (DESIGN.md › Colors): gradiente do accent do jogo em foco + arte à direita, esmaecida para o
/// accent. Duas camadas fazem crossfade a cada troca (animação de Opacity, barata no WPF).
/// </summary>
public sealed class BackdropView : Grid
{
    public static readonly DependencyProperty StateProperty =
        DependencyProperty.Register(nameof(State), typeof(BackdropState), typeof(BackdropView), new PropertyMetadata(null, (d, _) => ((BackdropView)d).Present()));

    public static readonly DependencyProperty IsLargeProperty =
        DependencyProperty.Register(nameof(IsLarge), typeof(bool), typeof(BackdropView), new PropertyMetadata(false, (d, _) => ((BackdropView)d).Present()));

    public static readonly DependencyProperty TransitionProperty =
        DependencyProperty.Register(nameof(Transition), typeof(TimeSpan), typeof(BackdropView), new PropertyMetadata(TimeSpan.FromMilliseconds(600)));

    private readonly Layer[] _layers = [new(), new()];
    private int _front;
    private BackdropState? _shown;
    private bool _shownLarge;

    public BackdropView()
    {
        IsHitTestVisible = false;
        foreach (var layer in _layers)
        {
            Children.Add(layer.Root);
        }
    }

    public BackdropState? State
    {
        get => (BackdropState?)GetValue(StateProperty);
        set => SetValue(StateProperty, value);
    }

    public bool IsLarge
    {
        get => (bool)GetValue(IsLargeProperty);
        set => SetValue(IsLargeProperty, value);
    }

    public TimeSpan Transition
    {
        get => (TimeSpan)GetValue(TransitionProperty);
        set => SetValue(TransitionProperty, value);
    }

    private void Present()
    {
        var state = State;
        if (state is null || (state == _shown && IsLarge == _shownLarge))
        {
            return;
        }

        // Mesmo jogo e mesma arte: só atualiza a camada da frente (ex.: arte terminou de carregar).
        bool sameGame = _shown?.GameId == state.GameId && _shown.Accent == state.Accent && IsLarge == _shownLarge;
        _shown = state;
        _shownLarge = IsLarge;
        if (sameGame)
        {
            _layers[_front].Show(state, IsLarge);
            return;
        }

        var back = _layers[1 - _front];
        var front = _layers[_front];
        back.Show(state, IsLarge);
        Panel.SetZIndex(back.Root, 1);
        Panel.SetZIndex(front.Root, 0);
        var duration = new Duration(Transition);
        back.Root.BeginAnimation(OpacityProperty, new DoubleAnimation(0, 1, duration));
        front.Root.BeginAnimation(OpacityProperty, new DoubleAnimation(1, 0, duration) { BeginTime = Transition });
        _front = 1 - _front;
    }

    /// <summary>Máscara opaca até <paramref name="solid"/> e transparente a partir de <paramref name="clear"/>.</summary>
    private static LinearGradientBrush Fade(Point start, Point end, double solid, double clear)
    {
        var brush = new LinearGradientBrush([new GradientStop(Colors.Black, solid), new GradientStop(Colors.Transparent, clear)], start, end);
        brush.Freeze();
        return brush;
    }

    /// <summary>Uma camada: tinta (gradiente), arte e esmaecimento para o accent.</summary>
    private sealed class Layer
    {
        private readonly Rectangle _tint = new();
        private readonly Rectangle _art = new();
        private readonly Border _artHost = new() { HorizontalAlignment = HorizontalAlignment.Right, VerticalAlignment = VerticalAlignment.Top };
        private readonly Rectangle _fadeLeft = new();
        private readonly Rectangle _fadeBottom = new();

        public Layer()
        {
            Root = new Grid { Opacity = 0 };
            Root.Children.Add(_tint);
            // A arte some para a esquerda e para baixo (máscaras aninhadas), como no protótipo.
            _artHost.Child = _art;
            _artHost.OpacityMask = Fade(new Point(1, 0), new Point(0, 0), 0.45, 0.92);
            _art.OpacityMask = Fade(new Point(0, 0), new Point(0, 1), 0.55, 1);
            Root.Children.Add(_artHost);
            Root.Children.Add(_fadeLeft);
            Root.Children.Add(_fadeBottom);
        }

        public Grid Root { get; }

        public void Show(BackdropState state, bool large)
        {
            _tint.Fill = new LinearGradientBrush(state.Accent, state.Deep, 70);
            _artHost.Width = large ? 1500 : 1240;
            _artHost.Height = large ? 1080 : 760;
            _art.Fill = state.Hero is null ? null : new ImageBrush(state.Hero)
            {
                Stretch = Stretch.UniformToFill,
                AlignmentX = state.HeroAlignment.X < 0.4 ? AlignmentX.Left : state.HeroAlignment.X > 0.6 ? AlignmentX.Right : AlignmentX.Center,
                AlignmentY = state.HeroAlignment.Y < 0.35 ? AlignmentY.Top : state.HeroAlignment.Y > 0.65 ? AlignmentY.Bottom : AlignmentY.Center,
            };
            var transparentAccent = Color.FromArgb(0, state.Accent.R, state.Accent.G, state.Accent.B);
            var transparentDeep = Color.FromArgb(0, state.Deep.R, state.Deep.G, state.Deep.B);
            _fadeLeft.Fill = new LinearGradientBrush(
                [new GradientStop(state.Accent, 0), new GradientStop(state.Accent, 0.2), new GradientStop(transparentAccent, 0.6)],
                new Point(0, 0), new Point(1, 0));
            _fadeBottom.Fill = new LinearGradientBrush(
                [new GradientStop(transparentDeep, 0.4), new GradientStop(state.Deep, 1)],
                new Point(0, 0), new Point(0, 1));
        }
    }
}
