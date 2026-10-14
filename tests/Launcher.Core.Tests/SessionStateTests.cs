using Console.Runtime;
using Xunit;

namespace Launcher.Core.Tests;

public class SessionStateTests
{
    [Fact]
    public void Idle_is_the_default_state() => Assert.Equal(SessionState.Idle, default(SessionState));
}
