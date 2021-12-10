#!/bin/bash
# Copyright 2020-2021 The Databend Authors.
# SPDX-License-Identifier: Apache-2.0.

SCRIPT_PATH="$(cd "$(dirname "$0")" >/dev/null 2>&1 && pwd)"
cd "$SCRIPT_PATH/../.." || exit

# Caveat: has to kill query first.
# `query` tries to remove its liveness record from meta before shutting down.
# If meta is stopped, `query` will receive an error that hangs graceful
# shutdown.
killall databend-query
sleep 3

killall databend-meta
sleep 3

for bin in databend-query databend-meta; do
	if test -n "$(pgrep $bin)"; then
		echo "The $bin is not killed. force killing."
		killall -9 $bin
	fi
done

# Temp debugging config:
# Set `mode` to open to test restarting a metasrv cluster.
# TODO(xp): remove this and there should be a standard test for this.
mode=boot
# mode=open

if [ "$mode" == "boot" ]; then
	echo "=== boot new metasrv cluster of 3"

	echo 'Start Meta service HA cluster(3 nodes)...'

	nohup ./target/debug/databend-meta \
		--single true \
		--id 1 \
		--raft-dir "./_meta1" \
		--metric-api-address 0.0.0.0:28100 \
		--admin-api-address 0.0.0.0:28101 \
		--flight-api-address 0.0.0.0:9191 \
		--log-dir ./_logs1 \
		--raft-api-port 28103 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 9191

	nohup ./target/debug/databend-meta \
		--id 2 \
		--raft-dir "./_meta2" \
		--metric-api-address 0.0.0.0:28200 \
		--admin-api-address 0.0.0.0:28201 \
		--flight-api-address 0.0.0.0:28202 \
		--log-dir ./_logs2 \
		--raft-api-port 28203 \
		--join 127.0.0.1:28103 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 28202

	nohup ./target/debug/databend-meta \
		--id 3 \
		--raft-dir "./_meta3" \
		--metric-api-address 0.0.0.0:28300 \
		--admin-api-address 0.0.0.0:28301 \
		--flight-api-address 0.0.0.0:28302 \
		--log-dir ./_logs3 \
		--raft-api-port 28303 \
		--join 127.0.0.1:28103 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 28302

else

	echo "=== start initialized metasrv cluster of 3"

	# In the `open` mode, id and raft-api-port are not needed.
	# They are already stored in raft store.

	nohup ./target/debug/databend-meta \
		--raft-dir "./_meta1" \
		--metric-api-address 0.0.0.0:28100 \
		--admin-api-address 0.0.0.0:28101 \
		--flight-api-address 0.0.0.0:9191 \
		--log-dir ./_logs1 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 9191

	nohup ./target/debug/databend-meta \
		--raft-dir "./_meta2" \
		--metric-api-address 0.0.0.0:28200 \
		--admin-api-address 0.0.0.0:28201 \
		--flight-api-address 0.0.0.0:28202 \
		--log-dir ./_logs2 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 28202

	nohup ./target/debug/databend-meta \
		--raft-dir "./_meta3" \
		--metric-api-address 0.0.0.0:28300 \
		--admin-api-address 0.0.0.0:28301 \
		--flight-api-address 0.0.0.0:28302 \
		--log-dir ./_logs3 \
		&
	python scripts/ci/wait_tcp.py --timeout 5 --port 28302

fi

echo 'Start DatabendQuery node-1'
nohup target/debug/databend-query -c scripts/deploy/config/databend-query-node-1.toml &

echo "Waiting on node-1..."
python scripts/ci/wait_tcp.py --timeout 5 --port 9091

echo 'Start DatabendQuery node-2'
nohup target/debug/databend-query -c scripts/deploy/config/databend-query-node-2.toml &

echo "Waiting on node-2..."
python scripts/ci/wait_tcp.py --timeout 5 --port 9092

echo 'Start DatabendQuery node-3'
nohup target/debug/databend-query -c scripts/deploy/config/databend-query-node-3.toml &

echo "Waiting on node-3..."
python scripts/ci/wait_tcp.py --timeout 5 --port 9093

echo "All done..."
