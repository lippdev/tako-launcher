using System.Text.RegularExpressions;

namespace ConsoleMode.Runtime.SessionTools;

/// <summary>Se o Tako pode medir o FPS agora (o PresentMon precisa do grupo "Usuários de log de desempenho").</summary>
public enum PerfLogStatus
{
    /// <summary>O usuário não está no grupo: ligar o contador pede o UAC uma vez.</summary>
    NotGranted,

    /// <summary>Já está no grupo, mas o Windows só põe o grupo no token no próximo login.</summary>
    NeedsSignOut,

    /// <summary>Token com o grupo (ou Tako elevado): o PresentMon roda.</summary>
    Ready,
}

/// <summary>O que o passo elevado (<c>Tako.exe --grant-perf-log-access</c>) deve fazer.</summary>
public enum PerfLogAction
{
    Grant,
    Revoke,
}

/// <summary>Pedido para o passo elevado, lido da linha de comando.</summary>
public sealed record PerfLogCommand(PerfLogAction Action, string UserSid);

/// <summary>
/// Regras puras da permissão do contador de FPS (ADR-0017). O usuário entra no grupo local
/// "Usuários de log de desempenho" pelo SID <see cref="GroupSid"/>, que é o mesmo em qualquer idioma do Windows.
/// O SID do usuário é lido no Tako NÃO elevado e passado ao passo elevado: quem digita no UAC a senha de outro
/// administrador elevaria como esse administrador, e o grupo iria para a conta errada.
/// </summary>
public static partial class PerfLogAccess
{
    /// <summary>BUILTIN\Performance Log Users.</summary>
    public const string GroupSid = "S-1-5-32-559";

    public const string GrantArgument = "--grant-perf-log-access";

    public const string RevokeArgument = "--revoke-perf-log-access";

    /// <summary>Usado pelo instalador: lê o SID do usuário, pede o UAC para o passo elevado e liga o contador.</summary>
    public const string SetupArgument = "--setup-fps-counter";

    /// <summary>Saída do passo elevado.</summary>
    public const int ExitOk = 0;

    public const int ExitInvalidArguments = 2;

    public const int ExitFailed = 3;

    /// <summary>ERROR_CANCELLED: a pessoa disse "Não" no UAC.</summary>
    public const int UacCancelled = 1223;

    /// <summary>
    /// Só SIDs de conta de usuário: locais e de domínio (S-1-5-21-…) e do Microsoft Entra ID (S-1-12-1-…). Recusa
    /// grupos e SIDs conhecidos (Todos, Usuários, Administradores), que dariam a permissão a muita gente de uma vez.
    /// </summary>
    public static bool IsUserSid(string? sid) => sid is not null && UserSid().IsMatch(sid);

    /// <summary>
    /// Reconhece <c>--grant-perf-log-access &lt;SID&gt;</c> e <c>--revoke-perf-log-access &lt;SID&gt;</c>. Nulo quando os
    /// argumentos não são esse pedido; <paramref name="invalid"/> verdadeiro quando são, mas o SID não serve.
    /// </summary>
    public static PerfLogCommand? TryParse(IReadOnlyList<string> args, out bool invalid)
    {
        ArgumentNullException.ThrowIfNull(args);
        invalid = false;
        for (int i = 0; i < args.Count; i++)
        {
            PerfLogAction? action = args[i] switch
            {
                _ when string.Equals(args[i], GrantArgument, StringComparison.OrdinalIgnoreCase) => PerfLogAction.Grant,
                _ when string.Equals(args[i], RevokeArgument, StringComparison.OrdinalIgnoreCase) => PerfLogAction.Revoke,
                _ => null,
            };
            if (action is null)
            {
                continue;
            }

            string? sid = i + 1 < args.Count ? args[i + 1].Trim() : null;
            if (!IsUserSid(sid))
            {
                invalid = true;
                return null;
            }

            return new PerfLogCommand(action.Value, sid!);
        }

        return null;
    }

    /// <summary>Argumentos do passo elevado (o SID é sempre do usuário que pediu, lido antes do UAC).</summary>
    public static string Arguments(PerfLogAction action, string userSid)
    {
        if (!IsUserSid(userSid))
        {
            throw new ArgumentException("SID de usuário inválido.", nameof(userSid));
        }

        return (action == PerfLogAction.Grant ? GrantArgument : RevokeArgument) + " " + userSid;
    }

    /// <param name="tokenHasGroup">O token deste processo já tem o grupo.</param>
    /// <param name="elevated">O Tako roda como administrador (o PresentMon funciona sem o grupo).</param>
    /// <param name="isMember">A conta está no grupo (lido do Windows), mesmo que o token ainda não tenha.</param>
    public static PerfLogStatus Decide(bool tokenHasGroup, bool elevated, bool isMember) =>
        tokenHasGroup || elevated ? PerfLogStatus.Ready
        : isMember ? PerfLogStatus.NeedsSignOut
        : PerfLogStatus.NotGranted;

    [GeneratedRegex(@"^S-1-(5-21(-\d{1,10}){4,}|12-1(-\d{1,10}){4,})$", RegexOptions.CultureInvariant)]
    private static partial Regex UserSid();
}

/// <summary>Resultado de pedir ou tirar a permissão (com o UAC no meio).</summary>
public enum PerfLogChange
{
    Done,
    Cancelled,
    Failed,
}

/// <summary>Permissão do contador de FPS para o usuário atual.</summary>
public interface IPerfLogAccess
{
    PerfLogStatus Check();

    /// <summary>Põe o usuário atual no grupo pelo UAC (o passo elevado recebe o SID lido aqui).</summary>
    Task<PerfLogChange> GrantAsync();

    /// <summary>Tira o usuário atual do grupo pelo UAC.</summary>
    Task<PerfLogChange> RevokeAsync();
}
