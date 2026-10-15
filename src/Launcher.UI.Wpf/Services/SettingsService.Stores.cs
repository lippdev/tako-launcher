using Launcher.Core.Library;

namespace Launcher.UI.Wpf.Services;

/// <summary>Configurações › Lojas (ADR-0019): cada loja pode ser desligada; todas começam ligadas.</summary>
public sealed partial class SettingsService
{
    private const string DisabledStoresKey = "DisabledStores";

    /// <summary>Chave da primeira versão da fonte Xbox (#40); "false" vira Xbox desligado.</summary>
    private const string LegacyXboxKey = "XboxLibraryEnabled";

    private readonly HashSet<GameStore> _disabledStores = [];

    /// <summary>Uma loja foi ligada ou desligada.</summary>
    public event EventHandler<GameStore>? StoreToggled;

    public bool IsStoreEnabled(GameStore store) => !_disabledStores.Contains(store);

    public void SetStoreEnabled(GameStore store, bool enabled)
    {
        bool changed = enabled ? _disabledStores.Remove(store) : _disabledStores.Add(store);
        if (!changed)
        {
            return;
        }

        Save(DisabledStoresKey, string.Join(',', _disabledStores.Order()));
        StoreToggled?.Invoke(this, store);
    }

    private void LoadStores(IReadOnlyDictionary<string, string> saved)
    {
        if (!saved.ContainsKey(DisabledStoresKey) && saved.TryGetValue(LegacyXboxKey, out string? xbox) && bool.TryParse(xbox, out bool on) && !on)
        {
            _disabledStores.Add(GameStore.Xbox);
        }

        foreach (string name in saved.GetValueOrDefault(DisabledStoresKey, string.Empty).Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries))
        {
            if (Enum.TryParse(name, ignoreCase: true, out GameStore store))
            {
                _disabledStores.Add(store);
            }
        }
    }
}
