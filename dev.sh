#!/bin/bash

cargo watch -i src/proto/hpv.rs -i src/proto/mod.rs -x fmt -x "test --all"