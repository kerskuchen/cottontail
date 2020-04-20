git submodule update --remote && ^
pushd cottontail && ^
git fetch && ^
git checkout master && ^
git pull origin master

popd
pause