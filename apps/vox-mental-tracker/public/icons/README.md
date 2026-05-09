# App Icons

Place a 1024×1024 PNG named `master.png` here before running `vox run scripts/generate-icons.vox`.

To generate a quick placeholder with ImageMagick:
```bash
magick -size 1024x1024 xc:#0d1b2a -fill white -draw "text 412,530 'VOX'" public/icons/master.png
```
