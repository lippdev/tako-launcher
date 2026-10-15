using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Security.Principal;
using ConsoleMode.Runtime.SessionTools;
using Microsoft.Extensions.Logging;

namespace ConsoleMode.Runtime.Windows.SessionTools;

/// <summary>
/// Permissão do contador de FPS (ADR-0017): o usuário entra no grupo local "Usuários de log de desempenho"
/// (<see cref="PerfLogAccess.GroupSid"/>). O SID do usuário é lido aqui, sem elevação, e o Tako.exe é aberto de novo
/// com o verbo <c>runas</c> (UAC) só para o passo <see cref="RunElevated"/>.
/// </summary>
public sealed partial class WindowsPerfLogAccess(string takoExePath, ILogger<WindowsPerfLogAccess> logger) : IPerfLogAccess
{
    private const int ErrorMemberNotInAlias = 1377;
    private const int ErrorMemberInAlias = 1378;

    public PerfLogStatus Check()
    {
        try
        {
            using var identity = WindowsIdentity.GetCurrent();
            var principal = new WindowsPrincipal(identity);
            bool tokenHasGroup = principal.IsInRole(new SecurityIdentifier(PerfLogAccess.GroupSid));
            bool elevated = principal.IsInRole(WindowsBuiltInRole.Administrator);
            bool member = !tokenHasGroup && !elevated && identity.User is { } user && IsMember(user);
            return PerfLogAccess.Decide(tokenHasGroup, elevated, member);
        }
        catch (Exception ex) when (ex is SystemException)
        {
            LogCheckFailed(logger, ex);
            return PerfLogStatus.NotGranted;
        }
    }

    public Task<PerfLogChange> GrantAsync() => RunWithUacAsync(PerfLogAction.Grant);

    public Task<PerfLogChange> RevokeAsync() => RunWithUacAsync(PerfLogAction.Revoke);

    /// <summary>SID do usuário deste processo (não elevado).</summary>
    public static string? CurrentUserSid()
    {
        using var identity = WindowsIdentity.GetCurrent();
        return identity.User?.Value;
    }

    /// <summary>
    /// O passo elevado: põe (ou tira) <paramref name="command"/>.UserSid no grupo. Devolve o código de saída do Tako.exe.
    /// O SID já foi validado como SID de usuário em <see cref="PerfLogAccess.TryParse"/>.
    /// </summary>
    public static int RunElevated(PerfLogCommand command)
    {
        ArgumentNullException.ThrowIfNull(command);
        try
        {
            string group = GroupName();
            var sid = new SecurityIdentifier(command.UserSid);
            byte[] binary = new byte[sid.BinaryLength];
            sid.GetBinaryForm(binary, 0);
            int result = Native.ChangeMember(group, binary, command.Action == PerfLogAction.Grant);
            return result is 0 or ErrorMemberInAlias or ErrorMemberNotInAlias ? PerfLogAccess.ExitOk : PerfLogAccess.ExitFailed;
        }
        catch (Exception ex) when (ex is SystemException)
        {
            return PerfLogAccess.ExitFailed;
        }
    }

    /// <summary>Nome do grupo no idioma deste Windows ("Usuários de log de desempenho", "Performance Log Users"...).</summary>
    private static string GroupName()
    {
        string account = new SecurityIdentifier(PerfLogAccess.GroupSid).Translate(typeof(NTAccount)).Value;
        int slash = account.IndexOf('\\', StringComparison.Ordinal);
        return slash >= 0 ? account[(slash + 1)..] : account;
    }

    /// <summary>A conta está no grupo, mesmo que o token deste login ainda não mostre.</summary>
    private static bool IsMember(SecurityIdentifier user)
    {
        foreach (var member in Native.Members(GroupName()))
        {
            if (member == user)
            {
                return true;
            }
        }

        return false;
    }

    private async Task<PerfLogChange> RunWithUacAsync(PerfLogAction action)
    {
        // Lido aqui, sem elevação: no UAC a pessoa pode digitar a senha de outro administrador.
        if (CurrentUserSid() is not { } sid || !PerfLogAccess.IsUserSid(sid))
        {
            LogNoUserSid(logger);
            return PerfLogChange.Failed;
        }

        var info = new ProcessStartInfo(takoExePath, PerfLogAccess.Arguments(action, sid))
        {
            UseShellExecute = true,
            Verb = "runas",
            WindowStyle = ProcessWindowStyle.Hidden,
        };
        try
        {
            using var process = Process.Start(info);
            if (process is null)
            {
                return PerfLogChange.Failed;
            }

            await process.WaitForExitAsync().ConfigureAwait(false);
            int code = process.ExitCode;
            LogElevatedFinished(logger, action, code);
            return code == PerfLogAccess.ExitOk ? PerfLogChange.Done : PerfLogChange.Failed;
        }
        catch (Win32Exception ex) when (ex.NativeErrorCode == PerfLogAccess.UacCancelled)
        {
            return PerfLogChange.Cancelled;
        }
        catch (Exception ex) when (ex is Win32Exception or InvalidOperationException)
        {
            LogElevatedFailed(logger, ex, action);
            return PerfLogChange.Failed;
        }
    }

    [LoggerMessage(Level = LogLevel.Warning, Message = "Permissão do contador de FPS não pôde ser conferida")]
    private static partial void LogCheckFailed(ILogger logger, Exception exception);

    [LoggerMessage(Level = LogLevel.Warning, Message = "SID do usuário atual não encontrado")]
    private static partial void LogNoUserSid(ILogger logger);

    [LoggerMessage(Level = LogLevel.Information, Message = "Passo elevado {Action} terminou com código {ExitCode}")]
    private static partial void LogElevatedFinished(ILogger logger, PerfLogAction action, int exitCode);

    [LoggerMessage(Level = LogLevel.Warning, Message = "Passo elevado {Action} não abriu")]
    private static partial void LogElevatedFailed(ILogger logger, Exception exception, PerfLogAction action);

    private static unsafe partial class Native
    {
        private const uint MaxPreferredLength = uint.MaxValue;

        /// <summary>NetLocalGroupAddMembers / NetLocalGroupDelMembers com LOCALGROUP_MEMBERS_INFO_0 (só o SID).</summary>
        public static int ChangeMember(string group, byte[] sid, bool add)
        {
            fixed (byte* sidPointer = sid)
            {
                nint info = (nint)sidPointer; // LOCALGROUP_MEMBERS_INFO_0 { PSID lgrmi0_sid }
                return (int)(add ? NetLocalGroupAddMembers(null, group, 0, &info, 1) : NetLocalGroupDelMembers(null, group, 0, &info, 1));
            }
        }

        /// <summary>SIDs dos membros diretos do grupo local.</summary>
        public static List<SecurityIdentifier> Members(string group)
        {
            var members = new List<SecurityIdentifier>();
            nint buffer = 0;
            try
            {
                uint status = NetLocalGroupGetMembers(null, group, 0, &buffer, MaxPreferredLength, out uint read, out _, 0);
                if (status != 0 || buffer == 0)
                {
                    return members;
                }

                var entries = (nint*)buffer;
                for (int i = 0; i < read; i++)
                {
                    if (entries[i] != 0)
                    {
                        members.Add(new SecurityIdentifier(entries[i]));
                    }
                }

                return members;
            }
            finally
            {
                if (buffer != 0)
                {
                    _ = NetApiBufferFree(buffer);
                }
            }
        }

        [LibraryImport("netapi32.dll", StringMarshalling = StringMarshalling.Utf16)]
        private static partial uint NetLocalGroupAddMembers(string? server, string group, uint level, nint* buffer, uint count);

        [LibraryImport("netapi32.dll", StringMarshalling = StringMarshalling.Utf16)]
        private static partial uint NetLocalGroupDelMembers(string? server, string group, uint level, nint* buffer, uint count);

        [LibraryImport("netapi32.dll", StringMarshalling = StringMarshalling.Utf16)]
        private static partial uint NetLocalGroupGetMembers(string? server, string group, uint level, nint* buffer, uint maxLength, out uint read, out uint total, nint resume);

        [LibraryImport("netapi32.dll")]
        private static partial uint NetApiBufferFree(nint buffer);
    }
}
