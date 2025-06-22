using Microsoft.Extensions.DependencyInjection;
using Spectre.Console.Cli;

namespace Wolfware.Moonlit.Cli.Injection;

public sealed class TypeResolver : ITypeResolver
{
  private readonly IServiceProvider _provider;

  public TypeResolver(IServiceProvider provider)
  {
    _provider = provider ?? throw new ArgumentNullException(nameof(provider));
  }

  public object? Resolve(Type? type)
  {
    return type == null ? null : _provider.GetRequiredService(type);
  }
}
