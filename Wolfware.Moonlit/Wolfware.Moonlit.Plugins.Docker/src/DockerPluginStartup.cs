using Microsoft.Extensions.DependencyInjection;
using Wolfware.Moonlit.Plugins.Docker.Middlewares;
using Wolfware.Moonlit.Plugins.Extensions;
using Wolfware.Moonlit.Plugins.Pipelines;

namespace Wolfware.Moonlit.Plugins.Docker;

public sealed class DockerPluginStartup : PluginStartup
{
  protected override void AddMiddlewares(IServiceCollection services)
  {
    services.AddMiddleware<BuildImage>("build");
    services.AddMiddleware<PushImage>("push");
  }
}
