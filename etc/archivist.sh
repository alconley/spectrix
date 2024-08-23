#!/bin/bash

RUNNO=$1

BINARYDIR=/home/spieker-group/Compass/144Sm6Lid_July_2023/DAQ/run_$RUNNO/UNFILTERED

ARCHIVE=/home/spieker-group/Experiments/144Sm6Lid_July_2023/WorkingDir/raw_binary/run_$RUNNO.tar.gz

echo "Running archivist for binary data in $BINARYDIR to archive $ARCHIVE..."

cd $BINARYDIR

tar -cvzf $ARCHIVE ./*.BIN

cd -

echo "Complete."