# osm-admin-hierarchies

![Kreuzberg](kreuzberg.png)

## Prepare

Download OSM bundle.

```bash
curl https://download.geofabrik.de/europe/germany/berlin-latest.osm.pbf -o berlin-latest.osm.pbf
```

Extract administrative boundaries from OSM to speed up processing.

```bash
brew install osmium-tool
osmium tags-filter berlin-latest.osm.pbf \
  r/boundary=administrative \
  -o berlin-boundaries.pbf
```

## Build

```bash
cargo build --release
```

## Create RTree

```bash
./target/release/build-rtree --bin rtree.bin --pbf berlin-boundaries.pbf
```

## Locate point

List boundaries.

```bash
./target/release/locate -b rtree.bin -l 13.4,52.5
boundary: Berlin, level: 4
boundary: Kreuzberg, level: 10
boundary: Friedrichshain-Kreuzberg, level: 9
```

Compile geojson file with boundaries.

```bash
./target/release/locate -b rtree.bin -l 13.4,52.5 -g boundaries.geojson
cat boundaries.geojson | pbcopy
# paste in geojson.io or similar
```

## Benchmark

The benchmark requires a pre-built rtree (w/ `build-rtree`) and a CSV file with locations (columns: id, lng, lat).

```bash
./target/release/bench -- single \
  --bin brandenburg-rtree.bin \
  --locs 4000_locs.csv \
  -m 16
took 648.188979ms for 4000 requests
took 672.047414ms for 4000 requests
took 648.485565ms for 4000 requests
```

## Web Service

The web service requires a pre-built rtree (w/ `build-rtree`). There are two routes. The `/bulk` endpoint accepts a CSV body with locations (columns: id, lng, lat):

* `GET /locate?loc=LNG,LAT`
* `POST /bulk`

```bash
./target/release/service --bin rtree.bin
```

```bash
export LOC=13.425979614257812,52.53919655252312
curl "localhost:8080/locate?loc=$LOC"
{"names":["Berlin","Pankow","Prenzlauer Berg"]}
```
