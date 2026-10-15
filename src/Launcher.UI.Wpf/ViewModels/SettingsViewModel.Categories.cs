using System.Globalization;
using Launcher.Core.Library;
using Launcher.Core.Library.Sources;
using Launcher.Core.Updates;
using Launcher.UI.Wpf.Services;

namespace Launcher.UI.Wpf.ViewModels;

/// <summary>
/// As linhas de cada categoria das Configurações (ADR-0020). Textos em linguagem de usuário, sem IDs internos; as
/// chaves (<c>settings:...</c>) e as preferências salvas no <c>tako.db</c> continuam as mesmas de antes.
/// </summary>
public sealed partial class SettingsViewModel
{
    public const string SourceUrl = "https://github.com/" + ReleaseFeed.Repository;

    // ---------------------------------------------------------------- Geral

    private List<SettingRowViewModel> GeneralRows() =>
    [
        Row("startup", SettingKind.Toggle, "Abrir o Tako ao entrar no Windows", () => Settings.StartFullscreenOnLogin = !Settings.StartFullscreenOnLogin,
            description: () => StartupRegistration.IsSupported
                ? "Abre em tela cheia assim que você entra no Windows"
                : "Disponível nas versões instalada e portátil do Tako",
            isOn: () => Settings.StartFullscreenOnLogin),
        Row("language", SettingKind.Info, "Idioma", () => Shell.Toast("Por enquanto, o Tako está só em português"),
            description: () => "Outros idiomas chegam em versões futuras",
            value: () => "Português (Brasil)"),
        Header("updates", "Atualizações"),
        Row("updates", SettingKind.Action, "Procurar atualizações", () => Shell.CheckUpdatesNowCommand.Execute(null),
            description: () => Shell.UpdateStatus,
            value: () => Shell.IsUpdating ? "Procurando…" : "Procurar"),
        Row("autoupdate", SettingKind.Toggle, "Atualizar automaticamente", () => Shell.ToggleAutoUpdateCommand.Execute(null),
            description: () => "Procura e baixa a versão nova em segundo plano; instala ao reiniciar ou ao sair do Tako",
            isOn: () => Shell.UpdatePreferences.AutoUpdate),
        Row("prerelease", SettingKind.Toggle, "Receber versões de teste", () => Shell.TogglePrereleaseUpdatesCommand.Execute(null),
            description: () => "Versões alpha e beta chegam antes, mas podem ter falhas",
            isOn: () => Shell.UpdatePreferences.Prereleases),
    ];

    // ---------------------------------------------------------------- Biblioteca e lojas

    private List<SettingRowViewModel> LibraryRows() =>
    [
        Row("add", SettingKind.Action, "Adicionar jogo", Shell.AddGame,
            description: () => "Qualquer jogo ou atalho do PC; a capa e as informações vêm sozinhas",
            value: () => "Escolher"),
        Row("rescan", SettingKind.Action, "Procurar jogos instalados", Shell.RescanLibrary,
            description: () => "Lê de novo as lojas ligadas abaixo, sem precisar abrir nenhuma delas",
            value: () => "Procurar"),
        Header("stores", "Lojas"),
        StoreRow("steam", GameStore.Steam, "Steam", "Store.Steam"),
        StoreRow("epic", GameStore.Epic, "Epic Games", "Store.Epic"),
        StoreRow("gog", GameStore.Gog, "GOG", "Store.Gog"),
        StoreRow("xbox", GameStore.Xbox, "Xbox (PC Game Pass)", "Store.Xbox"),
        Header("organize", "Organização"),
        Row("collections", SettingKind.Action, "Coleções", Shell.ManageCollections,
            description: () => "Junte jogos do seu jeito; cada coleção vira uma fileira na Início e um filtro na biblioteca",
            value: () => CollectionsSummary),
        Row("metadata", SettingKind.Action, "Atualizar informações dos jogos", Shell.RefreshMetadata,
            description: () => "Descrição, gêneros, lançamento, notas e artes (loja da Steam e, se ligada, IGDB)",
            value: () => "Atualizar"),

        // Fontes de dados dos jogos (ADR-0021/0022): cada uma abre o próprio painel.
        Row("igdb", SettingKind.Action, "Dados dos jogos (IGDB)", () => Shell.OpenIgdbSettingsCommand.Execute(null),
            description: () => "Informações e artes de jogos de qualquer loja e de consoles, com a sua chave gratuita da Twitch",
            value: () => Shell.IgdbSummary),
        Row("screenscraper", SettingKind.Action, "Dados de ROMs (ScreenScraper)", () => Shell.OpenScreenScraperSettingsCommand.Execute(null),
            description: () => "Caixas, logos, vídeos e informações de jogos de console, reconhecidos pelo arquivo, com a sua conta gratuita",
            value: () => Shell.ScreenScraperSummary),
    ];

    /// <summary>"Criar", "1 coleção", "3 coleções".</summary>
    public string CollectionsSummary => Shell.Collections.Collections.Count switch
    {
        0 => "Criar",
        1 => "1 coleção",
        int n => string.Create(CultureInfo.InvariantCulture, $"{n} coleções"),
    };

    /// <summary>
    /// Loja (ADR-0019): ativar liga ou desliga; a descrição diz o que a última leitura achou e se falta o programa da
    /// loja (STORE-012). Desligada vale na hora, mesmo antes da próxima leitura.
    /// </summary>
    private SettingRowViewModel StoreRow(string key, GameStore store, string title, string icon) =>
        Row($"store:{key}", SettingKind.Toggle, title, () => Shell.ToggleStore(store),
            description: () => StoreStatus(store),
            isOn: () => Settings.IsStoreEnabled(store),
            icon: icon);

    public string StoreStatus(GameStore store) => !Settings.IsStoreEnabled(store)
        ? new LibrarySourceReport(store, Enabled: false, 0, ClientInstalled: false, Notice: null, Failed: false).Summary
        : Shell.LibrarySources.Reports.GetValueOrDefault(store) is { Enabled: true } report ? report.Summary : "Ainda não verificada; use \"Procurar jogos instalados\"";

    // ---------------------------------------------------------------- Aparência

    private List<SettingRowViewModel> AppearanceRows() =>
    [
        Row("themes", SettingKind.Action, "Tema", Shell.OpenThemes,
            description: () => "Escolha o tema, veja a prévia, importe temas do Playnite e remova os que não usa",
            value: () => Shell.CurrentThemeName),
        Row("dynamic", SettingKind.Toggle, "Cor dinâmica", () =>
            {
                Settings.DynamicColor = !Settings.DynamicColor;
                Shell.RefreshBackdrop();
            },
            description: () => "A tela assume a cor da arte do jogo em foco",
            isOn: () => Settings.DynamicColor),
        Row("motion", SettingKind.Toggle, "Reduzir animações", () => Settings.ReducedMotion = !Settings.ReducedMotion,
            description: () => "Tira as transições de cor, de rolagem e da barra lateral",
            isOn: () => Settings.ReducedMotion),
    ];

    // ---------------------------------------------------------------- Controles

    private List<SettingRowViewModel> ControlsRows() =>
    [
        Row("controller", SettingKind.Info, "Controle em uso", () => Shell.Toast(Shell.ControllerName is null ? "Conecte um controle Xbox ou PlayStation; o teclado continua funcionando" : $"{Shell.ControllerName} conectado"),
            description: () => "Controles Xbox e PlayStation (DualSense e DualShock 4) funcionam sem configurar nada",
            value: () => Shell.ControllerName ?? "Nenhum"),
        Header("shortcuts", "Central de Controle"),
        Row("shortcut-pad", SettingKind.Action, "Abrir a Central de Controle no controle", () => Shell.CaptureChordCommand.Execute(null),
            description: () => "Segure os botões por meio segundo durante o jogo; A para escolher outra combinação",
            value: () => Shell.ChordLabel),
        Row("shortcut-key", SettingKind.Action, "Abrir a Central de Controle no teclado", () => Shell.CaptureKeyCommand.Execute(null),
            description: () => "Funciona com o jogo na frente; use Ctrl ou Alt junto (o Shift+Tab é da Steam)",
            value: () => Shell.KeyLabel),
        Row("shortcut-reset", SettingKind.Action, "Restaurar atalhos padrão", () => Shell.ResetShortcutsCommand.Execute(null),
            description: () => Shell.ShortcutDefaultsLabel,
            value: () => "Restaurar"),
        Header("keyboard", "Teclado"),
        Row("shortcut-keys", SettingKind.Info, "Atalhos do teclado", () => Shell.Toast("Setas navegam · Enter seleciona · Esc volta · M abre o menu · Q e E trocam de seção · / busca"),
            description: () => "Setas navegam, Enter seleciona, Esc volta, M abre o menu, Q e E trocam de seção",
            value: () => "Ver atalhos"),
    ];

    // ---------------------------------------------------------------- Sistema

    private List<SettingRowViewModel> SystemRows() =>
    [
        Row("environment", SettingKind.Toggle, "Restaurar telas e som ao sair do jogo", () => Settings.RestoreEnvironmentAfterGame = !Settings.RestoreEnvironmentAfterGame,
            description: () => "Volta resolução, taxa, tela principal, saída de áudio e volume como estavam antes do jogo",
            isOn: () => Settings.RestoreEnvironmentAfterGame),
        Header("diagnostics", "Diagnóstico"),
        Row("diagnostics", SettingKind.Action, "Diagnóstico do sistema", Shell.ShowSystemDiagnostics,
            description: () => "Relatório do PC sem dados pessoais, para anexar ao relatar um problema",
            value: () => "Gerar"),

        Row("controller-test", SettingKind.Action, "Teste de controle", Shell.ShowControllerTest,
            description: () => "Mostra cada botão e analógico ao vivo e o que o controle envia; copie e mande na issue se algum botão falhar",
            value: () => "Abrir"),
        Row("logs", SettingKind.Action, "Registros do Tako", OpenLogs,
            description: () => "Registros dos últimos 7 dias, sem dados pessoais; abre a pasta no Windows",
            value: () => "Abrir pasta"),
        Header("recovery", "Recuperação"),
        Row("safemode", SettingKind.Action, "Modo seguro", Shell.ExitSafeMode,
            description: () => Shell.SafeModeDescription,
            value: () => Shell.SafeModeLabel),
        Row("fse", SettingKind.Info, "Tela cheia do Windows (Xbox mode)", () => Shell.ShowFseStatusCommand.Execute(null),
            description: () => Shell.FseDescription,
            value: () => Shell.FseStatus.Label),
        Header("advanced", "Avançado"),
        Row("developer", SettingKind.Toggle, "Opções de desenvolvedor", () => Settings.DeveloperOptions = !Settings.DeveloperOptions,
            description: () => "Ferramentas para quem cria temas ou testa o Tako; deixe desligado no dia a dia",
            isOn: () => Settings.DeveloperOptions),
        Row("theme", SettingKind.Action, "Recarregar o tema do Tako", ReapplyTheme,
            description: () => "Lê de novo os arquivos do tema padrão",
            value: () => "Recarregar",
            visible: () => Settings.DeveloperOptions),
        Row("playnite", SettingKind.Action, "Prévia do tema Playnite embutido", Shell.ShowPlaynitePreview,
            description: () => "Mostra a sua biblioteca no tema Fullscreen do Playnite que vem com o Tako",
            value: () => "Abrir",
            visible: () => Settings.DeveloperOptions),
        Row("crash", SettingKind.Action, "Testar a tela de recuperação", SimulateThemeFailure,
            description: () => "Carrega um tema com erro de propósito para ver o Tako se recuperar",
            value: () => "Testar",
            visible: () => Settings.DeveloperOptions),
    ];

    /// <summary>Opções de desenvolvedor › Recarregar o tema do Tako.</summary>
    public void ReapplyTheme()
    {
        var result = Themes.Apply(ThemeService.DefaultTheme);
        Shell.Toast(result.Succeeded ? "Tema do Tako recarregado" : $"Não foi possível recarregar o tema: {result.Error}");
        Shell.NotifySettingsChanged();
    }

    /// <summary>Opções de desenvolvedor › Testar a tela de recuperação.</summary>
    public void SimulateThemeFailure()
    {
        Shell.ShowRecovery(Themes.Apply(ThemeService.InvalidSampleTheme));
        Shell.NotifySettingsChanged();
    }

    /// <summary>Abre no Explorer a pasta dos registros de diagnóstico (MVP-033).</summary>
    private void OpenLogs()
    {
        try
        {
            System.IO.Directory.CreateDirectory(AppPaths.LogsDirectory);
            System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo(AppPaths.LogsDirectory) { UseShellExecute = true })?.Dispose();
        }
        catch (Exception ex) when (ex is System.ComponentModel.Win32Exception or System.IO.IOException or UnauthorizedAccessException)
        {
            Shell.Toast($"Não foi possível abrir a pasta de registros: {ex.Message}");
        }
    }

    // ---------------------------------------------------------------- Sobre

    private List<SettingRowViewModel> AboutRows() =>
    [
        Row("version", SettingKind.Info, "Versão", () => Shell.Toast($"Tako {UpdateService.CurrentVersion} · versão {UpdateService.InstallKind}"),
            description: () => $"Versão {UpdateService.InstallKind}",
            value: () => $"Tako {UpdateService.CurrentVersion}"),
        Row("license", SettingKind.Action, "Licença", ShowLicense,
            description: () => "Software livre: você pode usar, estudar, mudar e compartilhar o Tako",
            value: () => "AGPL-3.0"),
        Row("notices", SettingKind.Action, "Créditos e avisos de terceiros", ShowNotices,
            description: () => "Projetos de outras pessoas que o Tako usa, com as licenças deles",
            value: () => "Ver"),
        Row("source", SettingKind.Action, "Código-fonte", OpenSource,
            description: () => "Todo o código do Tako, aberto para quem quiser ver ou ajudar",
            value: () => "github.com/" + ReleaseFeed.Repository),
    ];

    private void ShowLicense() => ShowText(
        "Licença AGPL-3.0",
        "O Tako é software livre, distribuído sob a GNU Affero General Public License, versão 3 (AGPL-3.0-only). Você pode "
        + "usar, estudar, mudar e compartilhar o Tako. Quem distribuir uma versão modificada, inclusive como serviço pela "
        + "rede, precisa oferecer o código-fonte dela sob a mesma licença. O texto completo está no arquivo LICENSE do código-fonte.");

    private void ShowNotices() => ShowText(
        "Créditos e avisos de terceiros",
        "• Console Mode (lippdev): telas, áudio, TV e ferramentas da sessão, sob a AGPL-3.0\n"
        + "• Intel PresentMon: medição do FPS, licença MIT\n"
        + "• CapFrameX: cálculo do FPS e do 1% low, licença MIT\n"
        + "• PyWebOSTV: pareamento com TVs LG, licença MIT\n"
        + "• Tema Fullscreen padrão do Playnite (Josef Nemec): licença MIT\n"
        + "• Fontes Inter e Source Sans 3: SIL Open Font License\n"
        + "• Logos das lojas: Simple Icons (CC0); as marcas são dos respectivos donos\n"
        + "Os textos completos ficam no arquivo NOTICE.md, ao lado do Tako.exe.");

    private void OpenSource()
    {
        if (!Shell.SessionTools.OpenPage(SourceUrl))
        {
            Shell.Toast($"Código-fonte em {SourceUrl}");
        }
    }

    private void ShowText(string title, string text)
    {
        DialogViewModel? dialog = null;
        dialog = new DialogViewModel(title, text, [new OverlayAction("Fechar", ActionStyle.Primary, new CommunityToolkit.Mvvm.Input.RelayCommand(() => Shell.CloseOverlay(dialog!)), IsDefault: true)]);
        Shell.PushOverlay(dialog);
    }
}
