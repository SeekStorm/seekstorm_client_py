Rem Copy files from seekstorm_rust_shard to seekstorm
Rem copy seekstorm\Cargo.toml  ..\SeekStorm\seekstorm
copy seekstorm\src\*.rs  ..\SeekStorm\seekstorm\src
copy seekstorm_server\README.md  ..\SeekStorm\seekstorm_server
copy seekstorm_server\Cargo.toml  ..\SeekStorm\seekstorm_server
copy seekstorm_server\src\*.*  ..\SeekStorm\seekstorm_server\src
copy seekstorm_client\README.md  ..\SeekStorm\seekstorm_client
copy seekstorm_client\Cargo.toml  ..\SeekStorm\seekstorm_client
copy seekstorm_client\src\*.*  ..\SeekStorm\seekstorm_client\src
copy seekstorm_client\tests\*.*  ..\SeekStorm\seekstorm_client\tests
copy tests\*.rs  ..\SeekStorm\tests
copy .cargo\config.toml  ..\SeekStorm\.cargo
copy rustfmt.toml  ..\SeekStorm
copy README.md  ..\SeekStorm
copy CHANGELOG.md  ..\SeekStorm
copy ARCHITECTURE.md  ..\SeekStorm
copy FACETED_SEARCH.md  ..\SeekStorm
copy NGRAM_SEARCH.md  ..\SeekStorm