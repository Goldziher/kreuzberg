$ErrorActionPreference = "Stop"

$workspace = ridk exec bash -lc "cygpath -au '$env:GITHUB_WORKSPACE'"
$gemdir = "$workspace/packages/ruby"

ridk exec bash -lc "cd $gemdir && export RUSTUP_TOOLCHAIN=stable-gnu CC=x86_64-w64-mingw32-gcc CXX=x86_64-w64-mingw32-g++ && bundle exec rake clean && bundle exec rake build"
