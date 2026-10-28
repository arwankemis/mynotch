# Weather artwork

MGC Weather Icons by Richard / MingCute.

Community source: https://www.figma.com/community/file/1266650798229888389/weather-icons
Creator's site: https://mgcweather.framer.website/

The user supplied `Icon.svg` and `color_fill.svg` on 2026-10-28. They are preserved as `gradient-source.svg` and `compact-source.svg`. The public Community listing identifies its license as CC BY 4.0: https://creativecommons.org/licenses/by/4.0/. This notice attributes the supplied Community artwork; it does not claim rights to separate paid products on the creator's site.

Changes: select 21 symbols, restore transparency from the gradient sheet's black matte, normalize padding/scale, and pack transparent PNG atlases. Gradient source contains a flattened 2x raster sheet plus vector sun/moon; compact source contains a transparent 2x raster sheet and a vector title. These exports are not individual vector icons.

Regenerate with `node scripts/export-weather-icons.cjs [path-to-installed-sharp]`. Node/sharp are authoring tools only; no runtime dependency is added. The application includes the generated PNG bytes and uses Windows WIC/Direct2D.
