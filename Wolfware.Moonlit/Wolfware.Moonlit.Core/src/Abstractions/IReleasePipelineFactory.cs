using Wolfware.Moonlit.Core.Configuration;
using Wolfware.Moonlit.Core.Pipeline;

namespace Wolfware.Moonlit.Core.Abstractions;

public interface IReleasePipelineFactory
{
  ReleasePipeline Create(ReleaseConfiguration configuration);
}
