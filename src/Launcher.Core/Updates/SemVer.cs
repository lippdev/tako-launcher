using System.Globalization;
using System.Text.RegularExpressions;

namespace Launcher.Core.Updates;

/// <summary>Versão semântica mínima: 0.1.0, 0.1.0-alpha.5 (pré-release vem antes da final). Igual à do Console Mode.</summary>
public sealed partial class SemVer
{
    private readonly int[] _core;
    private readonly string[] _pre;

    private SemVer(int[] core, string[] pre)
    {
        _core = core;
        _pre = pre;
    }

    public bool IsPrerelease => _pre.Length > 0;

    public static SemVer Parse(string? text) => TryFind(text) ?? new SemVer([0, 0, 0], []);

    /// <summary>Primeira versão encontrada no texto ("v0.1.0-alpha.5", "Tako 0.1.0").</summary>
    public static SemVer? TryFind(string? text)
    {
        if (string.IsNullOrWhiteSpace(text))
        {
            return null;
        }

        var match = Pattern().Match(text);
        if (!match.Success)
        {
            return null;
        }

        int[] core =
        [
            int.Parse(match.Groups[1].Value, CultureInfo.InvariantCulture),
            int.Parse(match.Groups[2].Value, CultureInfo.InvariantCulture),
            match.Groups[3].Success ? int.Parse(match.Groups[3].Value, CultureInfo.InvariantCulture) : 0,
        ];
        string[] pre = match.Groups[4].Success ? match.Groups[4].Value.Split('.') : [];
        return new SemVer(core, pre);
    }

    public int CompareTo(SemVer? other)
    {
        if (other is null)
        {
            return 1;
        }

        for (int i = 0; i < 3; i++)
        {
            int c = _core[i].CompareTo(other._core[i]);
            if (c != 0)
            {
                return c;
            }
        }

        if (_pre.Length == 0 || other._pre.Length == 0)
        {
            return other._pre.Length.CompareTo(_pre.Length);
        }

        for (int i = 0; i < Math.Min(_pre.Length, other._pre.Length); i++)
        {
            string a = _pre[i];
            string b = other._pre[i];
            int c = int.TryParse(a, NumberStyles.None, CultureInfo.InvariantCulture, out int ai) && int.TryParse(b, NumberStyles.None, CultureInfo.InvariantCulture, out int bi)
                ? ai.CompareTo(bi)
                : string.CompareOrdinal(a, b);
            if (c != 0)
            {
                return c;
            }
        }

        return _pre.Length.CompareTo(other._pre.Length);
    }

    public override string ToString() => $"{_core[0]}.{_core[1]}.{_core[2]}{(_pre.Length > 0 ? "-" + string.Join('.', _pre) : string.Empty)}";

    [GeneratedRegex(@"(\d+)\.(\d+)(?:\.(\d+))?(?:-([0-9A-Za-z.\-]+))?")]
    private static partial Regex Pattern();
}
