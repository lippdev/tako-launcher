namespace Launcher.Core.Library.Sources;

/// <summary>
/// Resultado da leitura de uma loja: os jogos instalados, se o cliente oficial está neste PC e um aviso curto para as
/// Configurações quando algo impede jogar (ex.: "Epic Games Launcher não encontrado", STORE-012).
/// </summary>
public sealed record LibraryScan(IReadOnlyList<Game> Games, bool ClientInstalled, string? Notice = null)
{
    public static LibraryScan Missing(string notice) => new([], ClientInstalled: false, notice);
}

/// <summary>
/// Fonte de biblioteca (ADR-0019): uma loja que lista os próprios jogos instalados direto do disco ou do registro,
/// sem a loja aberta. Steam, Epic, GOG e Xbox se registram igual em <see cref="LibrarySources"/>.
/// </summary>
public interface ILibrarySource
{
    GameStore Store { get; }

    /// <summary>
    /// Lê os jogos instalados. Pode lançar <see cref="IOException"/> ou <see cref="UnauthorizedAccessException"/>:
    /// <see cref="LibrarySources"/> isola a falha (a loja some da lista e as outras seguem).
    /// </summary>
    LibraryScan Scan();
}

/// <summary>Leitura do registro do Windows atrás de interface: os testes usam um registro falso.</summary>
public interface IRegistryReader
{
    /// <summary>Subchaves de <paramref name="keyPath"/> (ex.: <c>HKEY_LOCAL_MACHINE\SOFTWARE\...</c>); vazio se não existe.</summary>
    IReadOnlyList<string> SubKeyNames(string keyPath);

    /// <summary>Valor de texto; nulo se a chave ou o valor não existem.</summary>
    string? GetString(string keyPath, string valueName);
}
