# Deep Interview Transcript — foreground-cmd-space

- Profile: standard
- Context type: brownfield
- Final ambiguity: 13%
- Threshold: 20%

## Round 1
**Q:** When you press Cmd+Space, do you want PhotonCast to become the true frontmost app every time — even when Show in Dock is off — like Spotlight/Raycast, or do you want a different behavior for dock-hidden mode?

**A:** yes, i want PhotonCast to become the true frontmost app every time — even when Show in Dock is off

## Round 2
**Q:** What exact user-visible behavior should count as “fixed” for Cmd+Space: should PhotonCast both become the frontmost app and put the launcher window/key focus visibly on top of all normal apps, even if that means changing current dock-hidden activation semantics?

**A:** become the frontmost app and put the launcher window/key focus visibly on top of all normal apps

## Round 3
**Q:** To keep this tight: should this fix be limited to Cmd+Space / launcher foregrounding only, and explicitly not change unrelated behavior like Dock presence, menu bar visibility, startup flow, or other secondary windows unless that is strictly required?

**A:** this fix be limited to Cmd+Space / launcher foregrounding only
