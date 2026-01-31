# https://github.com/casey/just

alias u := update
alias b := build

# Build the project in release mode
build: 
    cargo build --release

# update the version number (x.y.z | patch | minor | major) for app
update VER:
    ./update-version {{VER}}