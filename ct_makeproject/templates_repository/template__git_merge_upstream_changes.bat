git fetch origin master
git fetch cottontail master

git pull origin master
git subtree pull --prefix=cottontail cottontail master --squash
