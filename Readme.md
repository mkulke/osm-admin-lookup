# rs-geo-playground

## Prepare

```bash
brew install osmium-tool
osmium tags-filter berlin-latest.osm.pbf \
  r/boundary=administrative \
  -o berlin-boundaries.pbf
```

## Build

```bash
cargo build
```
