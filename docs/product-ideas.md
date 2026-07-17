# Product ideas

## Agent-recorded browser demos

Status: deferred product idea; no runtime capability, persistence, or execution is implied.

Explore an optional, review-oriented artifact that lets an agent demonstrate a completed UI change
by running a declared browser scenario and attaching the resulting video to the task/run.

- Use a structured accessibility-driven browser boundary such as Playwright MCP for navigation,
  interaction, screenshots, and observable verification.
- Consider a declarative storyboard format for a local development server, viewport, waits, and
  user-visible scenes; render a video only when the user or an approved workflow explicitly asks.
- Treat the video as evidence of one recorded local run, not proof of general correctness or a
  production deployment.
- Keep browser profile, credentials, network access, arbitrary script execution, and artifact
  retention behind explicit policy and user approval. Never expose or record secrets in a demo.
- Preserve the scenario, environment summary, generated video, and outcome as separate artifacts
  so reviewers can tell what was intended, what ran, and what was observed.

References:

- [Have your agent record video demos of its work with shot-scraper video](https://simonw.substack.com/p/have-your-agent-record-video-demos)
- [Playwright MCP documentation](https://playwright.dev/docs/getting-started-mcp)
