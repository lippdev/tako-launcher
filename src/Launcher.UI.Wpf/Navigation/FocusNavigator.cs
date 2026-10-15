using System.Runtime.CompilerServices;
using System.Windows;
using System.Windows.Controls.Primitives;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Media.Animation;
using Launcher.Core.Navigation;

namespace Launcher.UI.Wpf.Navigation;

/// <summary>
/// Navegação por controle (MVP-028/029): move o foco entre elementos com <see cref="Nav.GroupProperty"/>
/// pela posição na tela (<see cref="SpatialNavigator"/>), lembra o último item de cada grupo e rola o
/// conteúdo para manter o foco visível. Mesmo comportamento do protótipo HTML.
/// </summary>
public sealed class FocusNavigator(FrameworkElement stage, Func<IReadOnlyList<FrameworkElement>> scopes, Func<TimeSpan> scrollDuration)
{
    private const double SafeTop = 180;
    private const double SafeBottom = 1080 - 130;
    private const double RowRightMargin = 72;

    /// <summary>Folga do item focado nas fileiras "Reveal": anel de foco (10) e um respiro.</summary>
    private const double RevealPadding = 16;

    /// <summary>Deslocamento pedido por último a cada fileira "Reveal" (a animação pode estar no meio do caminho).</summary>
    private static readonly DependencyProperty RevealOffsetProperty =
        DependencyProperty.RegisterAttached("RevealOffset", typeof(double), typeof(FocusNavigator), new PropertyMetadata(0d));

    /// <summary>Folga entre o item focado e a borda de uma moldura com recorte (o anel de foco fica 10 para fora).</summary>
    private const double FrameMargin = 16;

    /// <summary>Destino de cada rolagem: no meio de uma animação o valor lido é o intermediário, não o destino.</summary>
    private readonly ConditionalWeakTable<TranslateTransform, StrongBox<double>> _targets = [];

    private readonly Dictionary<string, FocusMemory> _groupMemory = [];
    private FocusMemory? _lastOutsideSidebar;

    public event EventHandler<FrameworkElement?>? FocusChanged;

    public FrameworkElement? Focused { get; private set; }

    /// <summary>Elementos navegáveis e visíveis no escopo atual (sobreposição do topo, ou barra lateral + página).</summary>
    public IReadOnlyList<FrameworkElement> Navigables()
    {
        var result = new List<FrameworkElement>();
        foreach (var scope in scopes())
        {
            var modal = FindAll(scope, e => Nav.GetIsModalScope(e) && e.IsVisible).FirstOrDefault();
            result.AddRange(FindAll(modal ?? scope, IsNavigable));
        }

        return result;
    }

    public bool Move(NavDirection direction)
    {
        var all = Navigables();
        if (Focused is null || !all.Contains(Focused))
        {
            return FocusDefault(null);
        }

        string? group = Nav.GetGroup(Focused);
        bool horizontal = direction is NavDirection.Left or NavDirection.Right;
        var candidates = all;
        if (horizontal && IsHorizontallyLocked(Focused))
        {
            candidates = all.Where(e => Nav.GetGroup(e) == group).ToList();
        }

        // Ligação vertical entre grupos (cartão "audio" ↔ ícone "audio").
        if (!horizontal && Nav.GetLink(Focused) is string link)
        {
            var from = RectOf(Focused);
            var linked = all.FirstOrDefault(e => Nav.GetGroup(e) != group && Nav.GetLink(e) == link && IsInDirection(from, RectOf(e), direction));
            if (linked is not null)
            {
                Focus(linked);
                return true;
            }
        }

        // Sair da barra lateral para a direita volta exatamente para onde o usuário estava.
        if (group == "sidebar" && direction == NavDirection.Right && _lastOutsideSidebar?.Target is { } back && all.Contains(back))
        {
            Focus(back);
            return true;
        }

        var next = SpatialNavigator.FindNext(RectOf(Focused), candidates.Select(e => new NavCandidate<FrameworkElement>(e, RectOf(e), Nav.GetGroup(e) ?? string.Empty)), direction);
        if (next is null)
        {
            return false;
        }

        var target = next.Value.Item;
        string targetGroup = next.Value.Group;
        bool sidebarTransition = targetGroup == "sidebar" || group == "sidebar";
        var entry = targetGroup != group ? all.FirstOrDefault(e => Nav.GetGroup(e) == targetGroup && Nav.GetIsGroupEntry(e)) : null;
        if (entry is not null)
        {
            target = entry;
        }
        else if (targetGroup != group && (!horizontal || sidebarTransition) && Remembered(targetGroup) is { } remembered && all.Contains(remembered))
        {
            target = remembered;
        }

        Focus(target);
        return true;
    }

    /// <summary>Foca o elemento com a chave pedida; senão o marcado como padrão; senão o primeiro do conteúdo.</summary>
    public bool FocusDefault(string? key)
    {
        var all = Navigables();
        if (key is not null && !all.Any(e => MatchesKey(e, key)) && RealizeKey(key))
        {
            all = Navigables();
        }

        var target = (key is null ? null : all.FirstOrDefault(e => MatchesKey(e, key)))
            ?? all.FirstOrDefault(Nav.GetIsDefault)
            // Sem padrão, o foco entra no conteúdo (primeiro item de cima para baixo), não na barra lateral.
            ?? all.Where(e => Nav.GetGroup(e) != "sidebar").OrderBy(e => RectOf(e).Top).ThenBy(e => RectOf(e).Left).FirstOrDefault()
            ?? (all.Count > 0 ? all[0] : null);
        if (target is null)
        {
            Focused = null;
            FocusChanged?.Invoke(this, null);
            return false;
        }

        Focus(target);
        return true;
    }

    public void Focus(FrameworkElement element, bool scroll = true)
    {
        Focused = element;
        if (Nav.GetGroup(element) is string group)
        {
            var remembered = new FocusMemory(element);
            _groupMemory[group] = remembered;
            if (group != "sidebar")
            {
                _lastOutsideSidebar = remembered;
            }
        }

        if (!element.IsKeyboardFocused)
        {
            Keyboard.Focus(element);
        }

        if (scroll)
        {
            KeepVisible(element);
        }

        FocusChanged?.Invoke(this, element);
    }

    /// <summary>Sincroniza quando o foco mudou por fora (clique ou mouse).</summary>
    public void Sync(FrameworkElement element)
    {
        if (element != Focused && IsNavigable(element))
        {
            Focus(element, scroll: false);
        }
    }

    /// <summary>A: executa o comando do elemento focado.</summary>
    public void Activate()
    {
        if (Focused is ButtonBase { Command: { } command } button && command.CanExecute(button.CommandParameter))
        {
            command.Execute(button.CommandParameter);
        }
        else if (Focused is not null && Nav.GetActivateCommand(Focused) is { } activate && activate.CanExecute(Nav.GetActivateParameter(Focused)))
        {
            activate.Execute(Nav.GetActivateParameter(Focused));
        }
    }

    public static bool MatchesKey(FrameworkElement element, string key)
    {
        string? own = Nav.GetKey(element);
        return own is not null && (own == key || $"{Nav.GetGroup(element)}:{own}" == key);
    }

    private static bool IsNavigable(FrameworkElement e) => Nav.GetGroup(e) is not null && e.IsVisible && e.IsEnabled;

    private static bool IsInDirection(NavRect from, NavRect to, NavDirection direction) =>
        direction == NavDirection.Up ? to.CenterY < from.CenterY : to.CenterY > from.CenterY;

    private static bool IsHorizontallyLocked(DependencyObject element)
    {
        for (var d = element; d is not null; d = VisualTreeHelper.GetParent(d))
        {
            if (Nav.GetLockHorizontal(d))
            {
                return true;
            }
        }

        return false;
    }

    private FrameworkElement? Remembered(string group) =>
        _groupMemory.TryGetValue(group, out var remembered) && remembered.Target is { IsVisible: true } element ? element : null;

    /// <summary>Item fora da tela numa lista virtualizada: pede às listas do escopo que criem o elemento dele.</summary>
    private bool RealizeKey(string key)
    {
        bool realized = false;
        foreach (var scope in scopes())
        {
            foreach (var host in FindAll(scope, e => e is IVirtualNavHost).Cast<IVirtualNavHost>().ToList())
            {
                realized |= host.RealizeKey(key);
            }
        }

        if (realized)
        {
            stage.UpdateLayout();
        }

        return realized;
    }

    private NavRect RectOf(FrameworkElement element)
    {
        if (!stage.IsAncestorOf(element))
        {
            return default;
        }

        var bounds = element.TransformToAncestor(stage).TransformBounds(new Rect(0, 0, element.ActualWidth, element.ActualHeight));
        return new NavRect(bounds.Left, bounds.Top, bounds.Width, bounds.Height);
    }

    /// <summary>
    /// Fileiras: o item focado fica na primeira posição até o fim da lista. Página: a fileira do item fica entre
    /// o cabeçalho e a barra de dicas; o destaque/cabeçalho volta a página ao topo.
    /// </summary>
    private void KeepVisible(FrameworkElement element)
    {
        for (DependencyObject? d = VisualTreeHelper.GetParent(element), child = element; d is not null; child = d, d = VisualTreeHelper.GetParent(d))
        {
            if (d is not FrameworkElement host || Nav.GetScrollHost(host) is not string mode)
            {
                continue;
            }

            var translate = MutableTranslate(host);

            if (VisualTreeHelper.GetParent(host) is not FrameworkElement viewport)
            {
                continue;
            }

            if (mode == "Reveal")
            {
                // Central de Controle: anda só o necessário para o item caber inteiro, com folga nas bordas.
                double x = element.TransformToAncestor(host).Transform(default).X;
                double visible = viewport.ActualWidth - System.Windows.Controls.Canvas.GetLeft(host) - RowRightMargin;
                double offset = RowReveal.Offset(x, element.ActualWidth, host.ActualWidth, visible, RevealPadding, (double)host.GetValue(RevealOffsetProperty));
                host.SetValue(RevealOffsetProperty, offset);
                Animate(translate, TranslateTransform.XProperty, -offset);
            }
            else if (mode == "Horizontal")
            {
                double x = element.TransformToAncestor(host).Transform(default).X;
                double visible = viewport.ActualWidth - System.Windows.Controls.Canvas.GetLeft(host) - RowRightMargin;
                double max = Math.Max(0, host.ActualWidth - visible);
                Animate(translate, TranslateTransform.XProperty, -Math.Min(x, max));
            }
            else
            {
                // Âncora vertical: o filho direto do host que contém o elemento (a fileira inteira, com título).
                // Tudo em coordenadas do palco com a lista sem rolagem; a faixa visível desconta o cabeçalho, a barra
                // de dicas e a moldura da lista (ADR-0020: antes, uma lista que não começava no topo da tela deixava o
                // item focado atrás da barra de dicas).
                var row = (FrameworkElement)child;
                double y = TargetOf(translate);
                double origin = host.TransformToAncestor(stage).Transform(default).Y - translate.Y;
                var (bandTop, bandBottom) = Band(host);
                bool isFirst = (VisualTreeHelper.GetChildrenCount(host) > 0 && VisualTreeHelper.GetChild(host, 0) == row)
                    || FindAll(host, IsNavigable).FirstOrDefault() == element;
                double target = ScrollMath.Vertical(
                    y,
                    StageRect(row, host, origin),
                    StageRect(element, host, origin),
                    isFirst,
                    bandTop,
                    bandBottom);
                Animate(translate, TranslateTransform.YProperty, target);
            }
        }
    }

    /// <summary>Deslocamento vertical para onde a lista está indo (ou está, sem animação).</summary>
    private double TargetOf(TranslateTransform transform) =>
        _targets.TryGetValue(transform, out var target) ? target.Value : transform.Y;

    /// <summary>
    /// Faixa vertical visível para a lista, no palco: entre o cabeçalho e a barra de dicas e dentro da moldura com
    /// recorte mais próxima (ex.: a coluna de conteúdo das Configurações).
    /// </summary>
    private (double Top, double Bottom) Band(FrameworkElement host)
    {
        double top = SafeTop;
        double bottom = SafeBottom;
        for (var d = VisualTreeHelper.GetParent(host); d is not null && d != stage; d = VisualTreeHelper.GetParent(d))
        {
            if (d is FrameworkElement { ClipToBounds: true } frame && stage.IsAncestorOf(frame))
            {
                var bounds = frame.TransformToAncestor(stage).TransformBounds(new Rect(0, 0, frame.ActualWidth, frame.ActualHeight));
                top = Math.Max(top, bounds.Top + FrameMargin);
                bottom = Math.Min(bottom, bounds.Bottom - FrameMargin);
                break;
            }
        }

        return (top, Math.Max(top, bottom));
    }

    /// <summary>Retângulo de um elemento da lista no palco, como se a lista não estivesse rolada.</summary>
    private static NavRect StageRect(FrameworkElement element, FrameworkElement host, double origin)
    {
        double top = origin + element.TransformToAncestor(host).Transform(default).Y;
        return new NavRect(0, top, element.ActualWidth, element.ActualHeight);
    }

    /// <summary>Transformações declaradas em templates vêm congeladas; a primeira rolagem troca por uma cópia animável.</summary>
    private static TranslateTransform MutableTranslate(FrameworkElement host)
    {
        if (host.RenderTransform is TranslateTransform { IsFrozen: false } current)
        {
            return current;
        }

        var existing = host.RenderTransform as TranslateTransform;
        var mutable = new TranslateTransform(existing?.X ?? 0, existing?.Y ?? 0);
        host.RenderTransform = mutable;
        return mutable;
    }

    private void Animate(TranslateTransform transform, DependencyProperty property, double to)
    {
        if (property == TranslateTransform.YProperty)
        {
            _targets.AddOrUpdate(transform, new StrongBox<double>(to));
        }

        var duration = scrollDuration();
        if (duration == TimeSpan.Zero)
        {
            transform.BeginAnimation(property, null);
            transform.SetValue(property, to);
            return;
        }

        transform.BeginAnimation(property, new DoubleAnimation(to, duration) { EasingFunction = new CubicEase { EasingMode = EasingMode.EaseOut } });
    }

    /// <summary>
    /// Último elemento focado de um grupo. Em listas virtualizadas o elemento pode ter sido reaproveitado para outro item;
    /// a chave guardada evita voltar o foco para o item errado.
    /// </summary>
    private sealed class FocusMemory(FrameworkElement element)
    {
        private readonly WeakReference<FrameworkElement> _element = new(element);
        private readonly string? _key = Nav.GetKey(element);

        public FrameworkElement? Target => _element.TryGetTarget(out var element) && Nav.GetKey(element) == _key ? element : null;
    }

    private static IEnumerable<FrameworkElement> FindAll(DependencyObject root, Func<FrameworkElement, bool> predicate)
    {
        int count = VisualTreeHelper.GetChildrenCount(root);
        for (int i = 0; i < count; i++)
        {
            var child = VisualTreeHelper.GetChild(root, i);
            if (child is FrameworkElement fe)
            {
                if (fe.Visibility != Visibility.Visible)
                {
                    continue;
                }

                if (predicate(fe))
                {
                    yield return fe;
                }
            }

            foreach (var nested in FindAll(child, predicate))
            {
                yield return nested;
            }
        }
    }
}
