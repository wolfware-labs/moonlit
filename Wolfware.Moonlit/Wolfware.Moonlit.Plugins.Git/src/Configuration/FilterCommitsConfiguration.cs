using System.Text;
using System.Text.RegularExpressions;

namespace Wolfware.Moonlit.Plugins.Git.Configuration;

public sealed class FilterCommitsConfiguration
{
  public string? TagRegex { get; set; }

  public string? TagPrefix { get; set; }

  public string? TagSuffix { get; set; }

  public Regex GetTagRegex()
  {
    if (this.TagRegex != null)
    {
      return new Regex(this.TagRegex, RegexOptions.Compiled | RegexOptions.IgnoreCase);
    }

    var regexStringBuilder = new StringBuilder();
    if (!string.IsNullOrEmpty(this.TagPrefix))
    {
      regexStringBuilder.Append(Regex.Escape(this.TagPrefix));
    }

    regexStringBuilder.Append("[0-9]+.[0-9]+.[0-9]+");

    if (!string.IsNullOrEmpty(this.TagSuffix))
    {
      regexStringBuilder.Append(Regex.Escape(this.TagSuffix));
    }

    return new Regex(regexStringBuilder.ToString(), RegexOptions.Compiled | RegexOptions.IgnoreCase);
  }
}
