using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Pipeline;

namespace Wolfware.Moonlit.Plugins.Moonlit;

public sealed class MoonlitPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IServiceCollection services)
  {
    throw new NotImplementedException();
  }
}
