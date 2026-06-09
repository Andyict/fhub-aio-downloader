#!/bin/bash
pkill fhub
sleep 2
./target/release/fhub &
echo "Restarted"
