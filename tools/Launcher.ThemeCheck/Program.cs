using System.IO;
using Launcher.Themes.Xaml;

namespace Launcher.ThemeCheck;

/// <summary>
/// Ferramenta de spike (MVP-016): roda o validador XAML sobre uma pasta de tema extraída e resume as violações.
/// Uso: Launcher.ThemeCheck &lt;pasta-do-tema&gt; [--load]
/// </summary>
internal static class Program
{
    [STAThread]
    private static int Main(string[] args)
    {
        if (args.Length == 0 || !Directory.Exists(args[0]))
        {
            Console.Error.WriteLine("Uso: Launcher.ThemeCheck <pasta-do-tema> [--load]");
            return 2;
        }

        string root = Path.GetFullPath(args[0]);
        bool load = args.Contains("--load", StringComparer.OrdinalIgnoreCase);
        var validator = new XamlValidator(XamlSecurityPolicy.CreateDefault());
        var loader = new RestrictedXamlLoader(validator);
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
