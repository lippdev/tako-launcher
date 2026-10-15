using ConsoleMode.Runtime.SessionTools;
using Xunit;

namespace ConsoleMode.Runtime.Tests.SessionTools;

public class PerfLogAccessTests
{
    private const string LocalUser = "S-1-5-21-3623811015-3361044348-30300820-1013";
    private const string EntraUser = "S-1-12-1-1234567890-1234567890-1234567890-1234567890";

    [Fact]
    public void The_group_is_found_by_sid_not_by_its_translated_name() =>
        Assert.Equal("S-1-5-32-559", PerfLogAccess.GroupSid);

    [Theory]
    [InlineData(LocalUser, true)]
    [InlineData(EntraUser, true)]
    [InlineData("S-1-1-0", false)] // Todos
    [InlineData("S-1-5-32-545", false)] // Usuários
    [InlineData("S-1-5-32-544", false)] // Administradores
    [InlineData("S-1-5-18", false)] // SYSTEM
    [InlineData("S-1-5-21-1-2", false)] // incompleto
    [InlineData("S-1-5-21-1-2-3-4 & calc", false)]
    [InlineData("", false)]
    [InlineData(null, false)]
    public void Only_user_account_sids_are_accepted(string? sid, bool ok) =>
        Assert.Equal(ok, PerfLogAccess.IsUserSid(sid));

    [Fact]
    public void Grant_and_revoke_are_read_from_the_command_line()
    {
        var grant = PerfLogAccess.TryParse(["--grant-perf-log-access", LocalUser], out bool invalid);
        Assert.False(invalid);
        Assert.Equal(new PerfLogCommand(PerfLogAction.Grant, LocalUser), grant);

        var revoke = PerfLogAccess.TryParse(["--ui", "--REVOKE-perf-log-access", EntraUser], out _);
        Assert.Equal(new PerfLogCommand(PerfLogAction.Revoke, EntraUser), revoke);
    }

    [Theory]
    [InlineData("--grant-perf-log-access")]
    [InlineData("--grant-perf-log-access", "S-1-1-0")]
    [InlineData("--revoke-perf-log-access", "S-1-5-32-544")]
    public void A_bad_sid_is_refused_not_ignored(params string[] args)
    {
        Assert.Null(PerfLogAccess.TryParse(args, out bool invalid));
        Assert.True(invalid);
    }

    [Fact]
    public void Other_command_lines_are_not_this_request()
    {
        Assert.Null(PerfLogAccess.TryParse(["--ui", "--safe-mode"], out bool invalid));
        Assert.False(invalid);
    }

    [Fact]
    public void The_elevated_step_gets_the_sid_read_before_the_uac()
    {
        Assert.Equal("--grant-perf-log-access " + LocalUser, PerfLogAccess.Arguments(PerfLogAction.Grant, LocalUser));
        Assert.Equal("--revoke-perf-log-access " + LocalUser, PerfLogAccess.Arguments(PerfLogAction.Revoke, LocalUser));
        Assert.Throws<ArgumentException>(() => PerfLogAccess.Arguments(PerfLogAction.Grant, "S-1-1-0"));
    }

    [Theory]
    [InlineData(true, false, false, PerfLogStatus.Ready)]
    [InlineData(false, true, false, PerfLogStatus.Ready)] // Tako elevado
    [InlineData(false, false, true, PerfLogStatus.NeedsSignOut)] // no grupo, token antigo
    [InlineData(false, false, false, PerfLogStatus.NotGranted)]
    public void Permission_reads_the_token_first(bool tokenHasGroup, bool elevated, bool member, PerfLogStatus expected) =>
        Assert.Equal(expected, PerfLogAccess.Decide(tokenHasGroup, elevated, member));
}
