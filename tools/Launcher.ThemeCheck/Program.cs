using System.IO;
using Launcher.PlayniteCompat.P10;
using Launcher.Themes.Xaml;

namespace Launcher.ThemeCheck;

/// <summary>
/// Ferramenta de spike (MVP-016): roda o validador XAML sobre uma pasta de tema extraída e resume as violações.
/// Uso: Launcher.ThemeCheck &lt;pasta-do-tema&gt; [--load] [--p10] [--base &lt;pasta-do-tema-padrão&gt;]
/// <c>--p10</c> usa a política do adaptador Playnite 10 (controles e extensões que ele implementa). Com <c>--p10 --load</c>
/// o tema é composto como na prévia do launcher (<see cref="P10Theme.Load"/>), e <c>--base</c> aponta o Fullscreen
/// padrão do qual um tema parcial herda os arquivos que não tem (homologação, MVP-022).
/// </summary>
internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        if (args.Length == 0 || !Directory.Exists(args[0]))
        {
            Console.Error.WriteLine("Uso: Launcher.ThemeCheck <pasta-do-tema> [--load] [--p10] [--base <pasta-do-tema-padrão>]");
            return 2;
        }

        string root = Path.GetFullPath(args[0]);
        bool load = args.Contains("--load", StringComparer.OrdinalIgnoreCase);
        var policy = XamlSecurityPolicy.CreateDefault();
        if (args.Contains("--p10", StringComparer.OrdinalIgnoreCase))
        {
            policy = P10Theme.Policy(policy);
        }

        var validator = new XamlValidator(policy);
        var loader = new RestrictedXamlLoader(validator);
        if (load && args.Contains("--p10", StringComparer.OrdinalIgnoreCase))
        {
            int baseIndex = Array.FindIndex(args, a => string.Equals(a, "--base", StringComparison.OrdinalIgnoreCase));
            return Compose(root, loader, baseIndex >= 0 && baseIndex + 1 < args.Length ? args[baseIndex + 1] : null);
        }
        var byCode = new Dictionary<XamlViolationCode, int>();
        int files = 0;
        int accepted = 0;
        int loaded = 0;

        foreach (string path in Directory.EnumerateFiles(root, "*.xaml", SearchOption.AllDirectories).Order(StringComparer.Ordinal))
        {
            files++;
            string relative = Path.GetRelativePath(root, path).Replace('\\', '/');
            var result = validator.Validate(File.ReadAllText(path), relative, expectedRoot: "ResourceDictionary");
            if (result.IsValid)
            {
                accepted++;
                Console.WriteLine($"OK    {relative}");
                if (load && TryLoad(loader, root, relative))
                {
                    loaded++;
                }

                continue;
            }

            Console.WriteLine($"FALHA {relative}");
            foreach (var violation in result.Violations)
            {
                Console.WriteLine($"      {violation}");
                byCode[violation.Code] = byCode.GetValueOrDefault(violation.Code) + 1;
            }
        }

        Console.WriteLine();
        Console.WriteLine($"Arquivos: {files}; aceitos pela validação: {accepted}" + (load ? $"; carregados pelo WPF: {loaded}" : string.Empty));
        foreach (var (code, count) in byCode.OrderByDescending(p => p.Value))
        {
            Console.WriteLine($"  {code}: {count}");
        }

        return accepted == files ? 0 : 1;
    }

    /// <summary>Composição completa como na prévia: cada arquivo enxerga os anteriores (precisa de um Application).</summary>
    private static int Compose(string root, RestrictedXamlLoader loader, string? baseRoot)
    {
        _ = new System.Windows.Application();
        var report = P10Theme.Load(root, loader, baseRoot);
        Console.WriteLine($"{report.Name} {report.Version} · ThemeApiVersion {report.ThemeApiVersion}");
        foreach (var diagnostic in report.Manifest.Diagnostics)
        {
            Console.WriteLine($"  {diagnostic}");
        }

        foreach (string file in report.Loaded)
        {
            Console.WriteLine($"OK      {file}");
        }

        foreach (var (file, reason) in report.Rejected)
        {
            Console.WriteLine($"RECUSA  {file} — {reason}");
        }

        foreach (string file in report.Inherited)
        {
            Console.WriteLine($"HERDADO {file}");
        }

        foreach (string file in report.Ignored)
        {
            Console.WriteLine($"IGNORADO {file}");
        }

        Console.WriteLine();
        Console.WriteLine($"Carregados: {report.Loaded.Count} de {report.Total}; herdados do padrão: {report.Inherited.Count}; ignorados (fora do padrão): {report.Ignored.Count}");
        return report.Rejected.Count == 0 ? 0 : 1;
    }

    private static bool TryLoad(RestrictedXamlLoader loader, string root, string relative)
    {
        try
        {
            loader.LoadResourceDictionary(root, relative);
            return true;
        }
        catch (ThemeXamlRejectedException ex)
        {
            // Referências a recursos de outros arquivos do tema também falham aqui, porque cada arquivo é carregado isolado.
            Console.WriteLine($"      WPF: {ex.InnerException?.Message ?? ex.Message}");
            return false;
        }
    }
}
