IF NOT EXIST assets mkdir assets
IF NOT EXIST assets_copy mkdir assets_copy
IF NOT EXIST assets_executable mkdir assets_executable

cargo watch --watch assets --watch assets_copy --watch assets_executable --watch cottontail/ct_assetbaker --exec "run --package ct_assetbaker"