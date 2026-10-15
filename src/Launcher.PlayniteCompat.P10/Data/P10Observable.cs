using System.ComponentModel;
using System.Runtime.CompilerServices;
using System.Windows.Input;

namespace Launcher.PlayniteCompat.P10.Data;

/// <summary>Base dos objetos que os temas enxergam por binding (o adaptador não depende do CommunityToolkit).</summary>
public abstract class P10Observable : INotifyPropertyChanged
{
    public event PropertyChangedEventHandler? PropertyChanged;

    protected bool SetField<T>(ref T field, T value, [CallerMemberName] string? name = null)
    {
        if (EqualityComparer<T>.Default.Equals(field, value))
        {
            return false;
        }

        field = value;
        OnPropertyChanged(name);
        return true;
    }

    protected void OnPropertyChanged([CallerMemberName] string? name = null) => PropertyChanged?.Invoke(this, new PropertyChangedEventArgs(name));
}

/// <summary>Comando do adaptador. O tema só consegue executar comandos que o próprio adaptador expõe.</summary>
public sealed class P10Command(Action<object?> execute, Func<object?, bool>? canExecute = null) : ICommand
{
    public P10Command(Action execute)
        : this(_ => execute())
    {
    }

    public event EventHandler? CanExecuteChanged
    {
        add => CommandManager.RequerySuggested += value;
        remove => CommandManager.RequerySuggested -= value;
    }

    public bool CanExecute(object? parameter) => canExecute?.Invoke(parameter) ?? true;

    public void Execute(object? parameter)
    {
        if (CanExecute(parameter))
        {
            execute(parameter);
        }
    }
}
