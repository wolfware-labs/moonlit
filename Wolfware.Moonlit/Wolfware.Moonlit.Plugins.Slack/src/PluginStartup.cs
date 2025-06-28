using Microsoft.Extensions.Configuration;
using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Abstractions;

namespace Wolfware.Moonlit.Plugins.Slack;

public sealed class PluginStartup : IPluginStartup
{
  public void Configure(IServiceCollection services, IConfiguration configuration)
  {
    throw new NotImplementedException();
  }
}
