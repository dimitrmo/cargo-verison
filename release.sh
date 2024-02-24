#!/bin/bash

cargo run patch -m "Release %s

[skip ci]
"

git push origin master
git push origin master --tags
