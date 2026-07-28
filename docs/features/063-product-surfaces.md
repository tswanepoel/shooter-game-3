# Feature 063 - Product surfaces (Gate / Panel / Chrome)

Product chrome uses three presentation kinds. Existing join / role / character / loadout / score / spectate flows keep their **051**–**053** rules; this feature is how they are drawn. Soft pointer (**061**) owns Gate and Panel.

## Kinds

| Kind | Soft pointer | Scene | Role |
|------|--------------|-------|------|
| **Gate** | Armed | Optional | Exclusive full-canvas entry / pick |
| **Panel** | Armed | Visible around a floating card | In-room menu over the world |
| **Chrome** | Disarmed | Visible | HUD / roster / spectate / name floats |

At most one Gate or Panel is up. Chrome stacks (roster + names + spectate hint).

`MpPhase` is product flow. Kind is how that flow is shown.

### Mapping

| Flow | Kind | Layout |
|------|------|--------|
| Lobby / Connecting | **Gate** | Narrow centered column (~320–400px): room, name, Join, status |
| Role | **Gate** | Same shell family as join |
| Character | **Gate** | Full-canvas field; large hero slot holds today’s letter picks (kit art later) |
| Loadout / Spawn bench | **Panel** | Centered card over the scene (~half view, max-width ~720); today’s slot controls with card-ready spacing |
| Score roster | **Chrome** | Always-on corner roster |
| Spectate strip | **Chrome** | Corner strip |
| Remote name floats (**060**) | **Chrome** | As **060** |

## Colour / type

Kid-friendly FPS: bright field, vivid accents, chunky readable buttons.

| Token | Draft |
|-------|--------|
| Gate field | Light play-space (soft gradient or flat) |
| Panel card | Light opaque card |
| Accents | Two or three hues (sky primary, lime secondary, coral danger) |
| Buttons | Solid fill, high-contrast short labels (Join, Play, Spawn) |
| Body text | Dark on light, or white on saturated fill |

## Acceptance criteria

- Join / role / character are Gates; loadout bench is a Panel; score and spectate are Chrome.
- Join is narrow and centered; character Gate has a large hero region; loadout Panel is a large floating card with scene around it.
- Palette and buttons are bright, playful, and high-contrast.
- Gate / Panel use soft pointer (**061**); with neither up, Living look uses the session.
