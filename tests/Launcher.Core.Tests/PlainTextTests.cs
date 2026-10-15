using Launcher.Core.Presentation;
using Xunit;

namespace Launcher.Core.Tests;

/// <summary>Substituto seguro do HtmlTextView (MVP-018): HTML vira texto, nada é interpretado.</summary>
public class PlainTextTests
{
    [Theory]
    [InlineData(null, "")]
    [InlineData("", "")]
    [InlineData("Texto simples", "Texto simples")]
    [InlineData("<b>Negrito</b> e <i>itálico</i>", "Negrito e itálico")]
    [InlineData("Linha 1<br>Linha 2<br/>Linha 3", "Linha 1\nLinha 2\nLinha 3")]
    [InlineData("<p>Um</p><p>Dois</p>", "Um\n\nDois")]
    [InlineData("<ul><li>A</li><li>B</li></ul>", "A\nB")]
    [InlineData("Tom &amp; Jerry &lt;3 &quot;ok&quot; &#233;", "Tom & Jerry <3 \"ok\" é")]
    [InlineData("a < b e c > d", "a < b e c > d")]
    [InlineData("  muitos\n\n   espaços\t aqui  ", "muitos espaços aqui")]
    public void Converts_html_to_plain_text(string? html, string expected) => Assert.Equal(expected, PlainText.FromHtml(html));

    [Theory]
    [InlineData("<script>alert('x')</script>Visível", "Visível")]
    [InlineData("<style>body{color:red}</style>Visível", "Visível")]
    [InlineData("<img src=\"https://rastreio.example/p.gif\" onerror=\"x()\">Visível", "Visível")]
    [InlineData("<a href=\"https://example.com\">link</a>", "link")]
    [InlineData("<iframe src=\"file:///C:/\"></iframe>Visível", "Visível")]
    [InlineData("<img alt='a > b' src=x>Visível", "Visível")]
    public void Drops_active_content_and_urls(string html, string expected)
    {
        string text = PlainText.FromHtml(html);
        Assert.Equal(expected, text);
        Assert.DoesNotContain("://", text, StringComparison.Ordinal);
    }

    [Fact]
    public void Limits_huge_descriptions()
    {
        string html = string.Concat(Enumerable.Repeat("<p>bloco de texto</p>", 10_000));
        Assert.True(PlainText.FromHtml(html).Length <= PlainText.MaxLength);
    }

    [Fact]
    public void Unclosed_script_hides_the_rest()
    {
        Assert.Equal("Antes", PlainText.FromHtml("Antes<script>while(true){}"));
    }
}
