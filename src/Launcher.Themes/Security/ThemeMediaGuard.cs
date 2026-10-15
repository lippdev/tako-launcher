using System.Collections;
using System.Diagnostics.CodeAnalysis;
using System.Globalization;
using System.IO;
using System.Reflection;
using System.Windows;
using System.Windows.Data;
using System.Windows.Input;
using System.Windows.Media;
using System.Windows.Navigation;
using Launcher.Core.Themes;

namespace Launcher.Themes.Security;

/// <summary>Uma origem de imagem, mídia, fonte ou cursor que a guarda bloqueou.</summary>
public sealed class ThemeMediaBlockedEventArgs(ThemeSourceBlock reason, Type targetType, string origin) : EventArgs
{
    public ThemeSourceBlock Reason { get; } = reason;

    /// <summary>Tipo da propriedade de destino (<see cref="ImageSource"/>, <see cref="Uri"/>, <see cref="FontFamily"/> ou <see cref="Cursor"/>).</summary>
    public Type TargetType { get; } = targetType;

    /// <summary>Resumo da origem sem pastas (só esquema e host, ou o nome do arquivo): pode ir para o log.</summary>
    public string Origin { get; } = origin;
}

/// <summary>
/// Defesa em tempo de execução para caminhos montados por dados em temas (MVP-016, questão aberta 8).
/// <para>
/// Um tema pode compor um texto em tempo de execução (<c>MultiBinding</c> com <c>StringFormat</c>, recursos
/// <c>sys:String</c>, <c>Tag</c>/<c>Text</c> de outros elementos) e ligá-lo a <c>Image.Source</c>,
/// <c>ImageBrush.ImageSource</c>, <c>MediaElement.Source</c> ou <c>BitmapImage.UriSource</c>. Quem transforma esse
/// texto em imagem é o conversor padrão do binding, e o <c>ImageSourceConverter</c> já abre o arquivo ou a URL durante
/// a conversão: coagir o valor na propriedade chegaria tarde demais (a conexão SMB com um caminho UNC, que vaza o hash
/// NTLM, já teria acontecido).
/// </para>
/// <para>
/// O único ponto por onde passam todas essas conversões é a tabela de conversores padrão do mecanismo de binding do
/// WPF (<c>DataBindEngine</c>, uma por thread de UI). A guarda troca, nessa tabela, os conversores de
/// <c>string</c>/<c>Uri</c>/<c>object</c> → <see cref="ImageSource"/>/<see cref="Uri"/>/<see cref="FontFamily"/>/<see cref="Cursor"/>
/// por um que primeiro consulta a <see cref="ThemeSourcePolicy"/> e só repassa ao conversor original a URI já
/// resolvida e permitida. O acesso é por reflexão a membros internos do WPF; se eles mudarem, a instalação falha e o
/// Tako não aplica temas Playnite (falha fechada), e o teste de fumaça acusa.
/// </para>
/// <para>
/// A troca vale para a thread inteira, não só para a árvore do tema, de propósito: um template do tema pode ser
/// instanciado fora do escopo esperado (Freezables como <c>ImageBrush</c> nem herdam propriedades da árvore), e a UI do
/// próprio Tako nunca converte texto em imagem (ela recebe <see cref="ImageSource"/> prontos do cache de artes), então
/// a guarda não muda nada para ela.
/// </para>
/// </summary>
public static class ThemeMediaGuard
{
    private static readonly Type[] SourceTypes = [typeof(string), typeof(Uri), typeof(object)];
    private static readonly Type[] TargetTypes = [typeof(ImageSource), typeof(Uri), typeof(FontFamily), typeof(Cursor)];
    private static readonly bool[] Directions = [false, true];

    [ThreadStatic]
    private static bool _installed;

    /// <summary>Origem bloqueada (silenciosamente para o usuário). O app registra no log, sem caminhos pessoais.</summary>
    public static event EventHandler<ThemeMediaBlockedEventArgs>? Blocked;

    /// <summary>Política consultada pela guarda (pastas de temas e cache de artes registradas pelo motor de temas).</summary>
    public static ThemeSourcePolicy Policy => ThemeSourcePolicy.Shared;

    /// <summary>
    /// Instala a guarda no mecanismo de binding da thread atual (a de UI). Idempotente. Deve rodar antes de o XAML do
    /// tema ser carregado; devolve falso (com o motivo) quando os membros internos do WPF não são os esperados.
    /// </summary>
    public static bool EnsureInstalled([NotNullWhen(false)] out string? problem)
    {
        problem = null;
        if (_installed)
        {
            return true;
        }

        try
        {
            Install();
            _installed = true;
            return true;
        }
        catch (Exception ex) when (ex is MissingMemberException or InvalidOperationException or TargetInvocationException or ArgumentException or InvalidCastException or TypeLoadException or NotSupportedException)
        {
            problem = $"Proteção de mídia dos temas indisponível ({ex.GetType().Name}: {ex.Message}).";
            return false;
        }
    }

    private static void Install()
    {
        const BindingFlags Internal = BindingFlags.NonPublic | BindingFlags.Instance;
        var engineType = typeof(FrameworkElement).Assembly.GetType("MS.Internal.Data.DataBindEngine", throwOnError: true)!; // throwOnError: nunca nulo
        object engine = engineType.GetProperty("CurrentDataBindEngine", BindingFlags.NonPublic | BindingFlags.Static)?.GetValue(null)
            ?? throw new MissingMemberException("DataBindEngine", "CurrentDataBindEngine");
        var getDefault = engineType.GetMethod("GetDefaultValueConverter", Internal, [typeof(Type), typeof(Type), typeof(bool)])
            ?? throw new MissingMemberException("DataBindEngine", "GetDefaultValueConverter");
        var table = engineType.GetField("_valueConverterTable", Internal)?.GetValue(engine) as IDictionary
            ?? throw new MissingMemberException("DataBindEngine", "_valueConverterTable");
        var keyType = engineType.GetNestedType("ValueConverterTableKey", BindingFlags.NonPublic)
            ?? throw new MissingMemberException("DataBindEngine", "ValueConverterTableKey");

        foreach (var source in SourceTypes)
        {
            foreach (var target in TargetTypes)
            {
                if (source == target)
                {
                    continue;
                }

                foreach (bool targetToSource in Directions)
                {
                    // Garante a entrada (cria o conversor original) e a substitui pela guarda.
                    if (getDefault.Invoke(engine, [source, target, targetToSource]) is not IValueConverter original || original is GuardedConverter)
                    {
                        continue;
                    }

                    object key = Activator.CreateInstance(keyType, Internal | BindingFlags.Public, binder: null, [source, target, targetToSource], CultureInfo.InvariantCulture)
                        ?? throw new MissingMemberException("DataBindEngine", "ValueConverterTableKey(.ctor)");
                    if (!table.Contains(key))
                    {
                        throw new InvalidOperationException("A chave da tabela de conversores do WPF mudou.");
                    }

                    table[key] = new GuardedConverter(original, target);
                    if (getDefault.Invoke(engine, [source, target, targetToSource]) is not GuardedConverter)
                    {
                        throw new InvalidOperationException("A tabela de conversores do WPF não aceitou a guarda.");
                    }
                }
            }
        }
    }

    /// <summary>
    /// Decide sobre um valor que vai virar imagem, mídia, fonte ou cursor. Exposto para os conversores do adaptador que
    /// também transformam texto em imagem.
    /// </summary>
    public static ThemeSourceDecision Evaluate(object? value, Type targetType, DependencyObject? target)
    {
        ArgumentNullException.ThrowIfNull(targetType);
        if (targetType == typeof(FontFamily))
        {
            // Nomes de família passam; qualquer local de fonte (arquivo, pasta, URI) montado por dados é recusado.
            return value is string family && family.IndexOfAny(['/', '\\', ':', '#', '%']) < 0
                ? new ThemeSourceDecision(null, ThemeSourceBlock.None)
                : ThemeSourceDecision.Deny(value is string text && LooksRemote(text) ? ThemeSourceBlock.Network : ThemeSourceBlock.External);
        }

        if (targetType == typeof(Cursor))
        {
            // Só os cursores do sistema; um .cur/.ani seria baixado ou lido pelo conversor.
            return value is string name && IsSystemCursor(name)
                ? new ThemeSourceDecision(null, ThemeSourceBlock.None)
                : ThemeSourceDecision.Deny(ThemeSourceBlock.External);
        }

        Uri? baseUri = BaseOf(target);
        return value switch
        {
            string text => Policy.Evaluate(text, baseUri),
            Uri uri => Policy.Evaluate(uri, baseUri),
            _ => ThemeSourceDecision.Deny(ThemeSourceBlock.Invalid),
        };
    }

    /// <summary>Avisa um bloqueio (o resumo não leva pastas).</summary>
    public static void Report(ThemeSourceBlock reason, Type targetType, object? value)
    {
        string origin = Summarize(value);
        Blocked?.Invoke(null, new ThemeMediaBlockedEventArgs(reason, targetType, origin));
    }

    private static Uri? BaseOf(DependencyObject? target)
    {
        if (target is null)
        {
            return null;
        }

        try
        {
            return BaseUriHelper.GetBaseUri(target);
        }
        catch (Exception ex) when (ex is InvalidOperationException or UriFormatException or ArgumentException)
        {
            return null;
        }
    }

    private static bool IsSystemCursor(string name)
    {
        string value = name.Trim();
        return value.Length > 0 && value.All(char.IsAsciiLetter) && Enum.TryParse<CursorType>(value, ignoreCase: true, out var type) && Enum.IsDefined(type);
    }

    private static bool LooksRemote(string text)
    {
        string value = text.Trim();
        return value.StartsWith(@"\\", StringComparison.Ordinal) || value.StartsWith("//", StringComparison.Ordinal);
    }

    /// <summary>Esquema e host para URIs; para caminhos, só o nome do arquivo (nada de pastas do usuário).</summary>
    private static string Summarize(object? value)
    {
        string text = value switch
        {
            Uri uri => uri.OriginalString,
            string s => s.Trim(),
            null => "(nulo)",
            _ => value.GetType().Name,
        };

        if (LooksRemote(text))
        {
            string host = text.TrimStart('\\', '/').Split('\\', '/')[0];
            return $@"\\{host}\…";
        }

        if (Uri.TryCreate(text, UriKind.Absolute, out var absolute) && !absolute.IsFile)
        {
            return absolute.IsAbsoluteUri && absolute.Host.Length > 0 ? $"{absolute.Scheme}://{absolute.Host}/…" : $"{absolute.Scheme}:…";
        }

        try
        {
            string name = Path.GetFileName(text.Replace('/', '\\').TrimEnd('\\'));
            return name.Length > 0 ? "…\\" + name : "(vazio)";
        }
        catch (ArgumentException)
        {
            return "(inválido)";
        }
    }

    /// <summary>
    /// Conversor padrão do binding com a guarda na frente: decide antes de o conversor original tocar o disco ou a rede,
    /// e entrega a ele a URI já resolvida (o que é carregado é exatamente o que foi permitido).
    /// </summary>
    private sealed class GuardedConverter(IValueConverter original, Type targetType) : IValueConverter
    {
        public object? Convert(object? value, Type type, object? parameter, CultureInfo culture)
        {
            if (value is not (string or Uri))
            {
                // ImageSource pronto (cache de artes, {ThemeFile}), bytes, nulo: o caminho original de sempre.
                return original.Convert(value, type, parameter, culture);
            }

            if (value is string text && string.IsNullOrWhiteSpace(text))
            {
                // Texto ainda vazio (binding montando o valor): nada a carregar nem a registrar.
                return targetType == typeof(FontFamily) ? DependencyProperty.UnsetValue : null;
            }

            var decision = Evaluate(value, targetType, parameter as DependencyObject);
            if (!decision.IsAllowed)
            {
                Report(decision.Block, targetType, value);
                return targetType == typeof(FontFamily) ? DependencyProperty.UnsetValue : null;
            }

            if (targetType == typeof(Uri))
            {
                return decision.Uri;
            }

            // Fontes e cursores: o nome original (já conferido). Imagens: a URI resolvida.
            object input = targetType == typeof(ImageSource) ? decision.Uri! : value; // imagem permitida sempre traz a URI
            return original.Convert(input, type, parameter, culture);
        }

        public object? ConvertBack(object? value, Type type, object? parameter, CultureInfo culture) =>
            original.ConvertBack(value, type, parameter, culture);
    }
}
