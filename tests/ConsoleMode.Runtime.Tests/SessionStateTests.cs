using ConsoleMode.Runtime;
using Xunit;

namespace ConsoleMode.Runtime.Tests;

public class SessionStateTests
{
    [Fact]
    public void Idle_is_the_default_state() => Assert.Equal(SessionState.Idle, default(SessionState));
}
