using ConsoleMode.Runtime;
using Launcher.Core.Session;
using Xunit;

namespace Launcher.Core.Tests;

public class SessionStateMachineTests
{
    // Tabela esperada (PRD §4.4): qualquer mudança na máquina precisa mudar este teste de regressão.
    private static readonly (SessionState From, SessionState To)[] Valid =
    [
        (SessionState.Idle, SessionState.EnteringFullscreen),
        (SessionState.Idle, SessionState.Browsing),
        (SessionState.EnteringFullscreen, SessionState.Browsing),
        (SessionState.EnteringFullscreen, SessionState.Error),
        (SessionState.Browsing, SessionState.LaunchingGame),
        (SessionState.Browsing, SessionState.Idle),
        (SessionState.LaunchingGame, SessionState.Playing),
        (SessionState.LaunchingGame, SessionState.Browsing),
        (SessionState.LaunchingGame, SessionState.Error),
        (SessionState.Playing, SessionState.Returning),
        (SessionState.Playing, SessionState.Error),
        (SessionState.Returning, SessionState.Restoring),
        (SessionState.Returning, SessionState.Error),
        (SessionState.Restoring, SessionState.Browsing),
        (SessionState.Restoring, SessionState.Error),
        (SessionState.Error, SessionState.Recovery),
        (SessionState.Recovery, SessionState.Browsing),
        (SessionState.Recovery, SessionState.DesktopFallback),
        (SessionState.DesktopFallback, SessionState.Idle),
    ];

    public static TheoryData<SessionState, SessionState> AllPairs()
    {
        var data = new TheoryData<SessionState, SessionState>();
        foreach (var from in Enum.GetValues<SessionState>())
        {
            foreach (var to in Enum.GetValues<SessionState>())
            {
                data.Add(from, to);
            }
        }

        return data;
    }

    [Theory]
    [MemberData(nameof(AllPairs))]
    public void Every_pair_of_states_follows_the_table(SessionState from, SessionState to)
    {
        var machine = new SessionStateMachine(from);
        SessionTransition? changed = null;
        SessionTransition? rejected = null;
        machine.Changed += (_, t) => changed = t;
        machine.Rejected += (_, t) => rejected = t;

        var result = machine.TryTransition(to);

        if (from == to)
        {
            // Idempotente: aceito, sem mudança e sem evento.
            Assert.True(result.IsNoOp);
            Assert.Null(changed);
            Assert.Null(rejected);
        }
        else if (Valid.Contains((from, to)))
        {
            Assert.True(result.Accepted);
            Assert.Equal(to, machine.State);
            Assert.Equal(new SessionTransition(from, to, true), changed);
            Assert.Null(rejected);
        }
        else
        {
            Assert.False(result.Accepted);
            Assert.Equal(from, machine.State);
            Assert.Null(changed);
            Assert.Equal(new SessionTransition(from, to, false), rejected);
        }

        Assert.Equal(from == to || Valid.Contains((from, to)), SessionStateMachine.CanTransition(from, to));
    }

    [Fact]
    public void Next_states_list_exactly_the_valid_transitions()
    {
        foreach (var from in Enum.GetValues<SessionState>())
        {
            Assert.Equal(Valid.Where(v => v.From == from).Select(v => v.To).Order(), SessionStateMachine.NextStates(from).Order());
        }
    }

    [Fact]
    public void A_full_game_session_goes_around_and_back_to_browsing()
    {
        var machine = new SessionStateMachine();
        var seen = new List<SessionState>();
        machine.Changed += (_, t) => seen.Add(t.To);

        foreach (var state in new[] { SessionState.EnteringFullscreen, SessionState.Browsing, SessionState.LaunchingGame, SessionState.Playing, SessionState.Returning, SessionState.Restoring, SessionState.Browsing })
        {
            Assert.True(machine.TryTransition(state).Accepted);
        }

        Assert.Equal([SessionState.EnteringFullscreen, SessionState.Browsing, SessionState.LaunchingGame, SessionState.Playing, SessionState.Returning, SessionState.Restoring, SessionState.Browsing], seen);
    }

    [Fact]
    public void Cancelling_or_failing_to_open_returns_to_browsing()
    {
        var machine = new SessionStateMachine(SessionState.Browsing);
        machine.TryTransition(SessionState.LaunchingGame);

        Assert.True(machine.TryTransition(SessionState.Browsing).Accepted);
        Assert.Equal(SessionState.Browsing, machine.State);
    }

    [Fact]
    public void Ending_a_game_twice_does_not_restore_twice()
    {
        var machine = new SessionStateMachine(SessionState.Playing);
        int changes = 0;
        machine.Changed += (_, _) => changes++;

        machine.TryTransition(SessionState.Returning);
        machine.TryTransition(SessionState.Returning);

        Assert.Equal(1, changes);
        Assert.False(machine.TryTransition(SessionState.Playing).Accepted);
        Assert.Equal(SessionState.Returning, machine.State);
    }

    [Fact]
    public void Starts_idle_by_default() => Assert.Equal(SessionState.Idle, new SessionStateMachine().State);
}
