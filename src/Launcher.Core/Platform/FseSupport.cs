namespace Launcher.Core.Platform;

/// <summary>Suporte ao Full Screen Experience (Xbox mode) neste PC.</summary>
public enum FseSupport
{
    /// <summary>Há sinal positivo de que o Windows oferece o Xbox mode aqui.</summary>
    Supported,

    /// <summary>Falta um requisito conhecido (Windows antigo, app Xbox ausente).</summary>
    Unavailable,

    /// <summary>Requisitos atendidos, mas sem como confirmar (rollout gradual, sinais ilegíveis).</summary>
    Indeterminate,
}

/// <summary>
/// Sinais coletados do Windows (os mesmos do spike MVP-007, <c>tools/Launcher.FseProbe</c>). Nulo = não foi possível ler.
/// </summary>
/// <param name="OsBuild">Build do Windows (<c>CurrentBuild</c> em <c>HKLM\...\Windows NT\CurrentVersion</c>).</param>
/// <param name="XboxAppInstalled">Pacote <c>Microsoft.GamingApp</c> (app Xbox, o app inicial padrão do Xbox mode) instalado.</param>
/// <param name="DeviceForm">Valor <c>DeviceForm</c> do OEM (46 = portátil de jogos, citado pela comunidade).</param>
public sealed record FseSignals(int? OsBuild, bool? XboxAppInstalled, int? DeviceForm);

public sealed record FseAssessment(FseSupport Support, string Reason)
{
    public string Label => Support switch
    {
        FseSupport.Supported => "Suportado",
        FseSupport.Unavailable => "Indisponível",
        _ => "Indeterminado",
    };
}

/// <summary>
/// Classificação do MVP-008. Não existe API oficial para saber se o Xbox mode está liberado, então a regra é
/// conservadora: só responde <see cref="FseSupport.Supported"/> com um sinal positivo (portátil de jogos, onde o modo foi
/// lançado para todos), nunca por exclusão. Desktops e notebooks recebem o modo aos poucos (Controlled Feature Rollout)
/// e ficam como <see cref="FseSupport.Indeterminate"/>.
/// </summary>
public static class FseClassifier
{
    /// <summary>Windows 11 24H2.</summary>
    public const int MinimumBuild = 26100;

    public const int HandheldDeviceForm = 46;

    public static FseAssessment Classify(FseSignals signals)
    {
        ArgumentNullException.ThrowIfNull(signals);
        return signals switch
        {
            { OsBuild: null } => new(FseSupport.Indeterminate, "Não foi possível ler a versão do Windows"),
            { OsBuild: < MinimumBuild } => new(FseSupport.Unavailable, $"Exige Windows 11 24H2 ou mais recente (este PC: build {signals.OsBuild})"),
            { XboxAppInstalled: false } => new(FseSupport.Unavailable, "O app Xbox não está instalado"),
            { XboxAppInstalled: null } => new(FseSupport.Indeterminate, "Não foi possível verificar o app Xbox"),
            { DeviceForm: HandheldDeviceForm } => new(FseSupport.Supported, "Portátil de jogos com Windows 11 24H2+ e app Xbox"),
            _ => new(FseSupport.Indeterminate, "Em desktops e notebooks a Microsoft libera o Xbox mode aos poucos e não há como confirmar"),
        };
    }
}
