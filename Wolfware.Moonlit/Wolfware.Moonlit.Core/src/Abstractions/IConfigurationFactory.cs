using Microsoft.Extensions.Configuration;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IConfigurationFactory
{
  public IConfiguration Create(Dictionary<string, string?> configurationData,
    IConfiguration? parentConfiguration = null);
}
