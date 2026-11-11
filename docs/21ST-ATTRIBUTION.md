# 21st.dev component attribution

`ui/src/components/SetupProgress.tsx` adapts the numbered step indicator, semantic list,
current-step state and completed-step navigation from **Stepper** by Sudharshan Pani.
The styling and the small string-based API were adapted for Open-Delay.

- Catalog: https://21st.dev/@sudharshanpanidev/components/stepper
- Original source: https://github.com/SudharshanPaniDev/flowpane/blob/main/src/components/Stepper/Stepper.tsx
- Original license: https://github.com/SudharshanPaniDev/flowpane/blob/main/LICENSE
- Retrieved: 2026-11-11

## Original license

MIT License

Copyright (c) 2026 Sudharshan Pani

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.


## Overlay workspace and introduction — design references (11 November 2026)

- Settings Sidebar Layout by felipemenezes098: https://21st.dev/@felipemenezes098/components/settings-2
- Onboarding Stages by Isaiah Bjork: https://21st.dev/@isaiahbjork/components/onboarding-stages

These public 21st.dev catalog pages informed the navigation grouping and staged introduction. Their component source was not imported: the settings registry URL returned 404, and the 21st MCP is not exposed in this session. OverlayEditor and WelcomeIntro are original implementations using existing Base UI controls, Lucide and Motion. No hosted AI generation was called. The existing SetupProgress adaptation retains its MIT notice above.
