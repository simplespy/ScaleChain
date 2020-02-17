package main

import (
	"fmt"
	"flag"
	"os"
)

func main() {
	var logCommand = flag.NewFlagSet("log", flag.ExitOnError)
	intervalFlag := logCommand.Uint("interval", 1, "set interval between log")
	durationFlag := logCommand.Uint("duration", 3600, "set interval between log")
	nodesFileFlag := logCommand.String("nodesFileFlag", "nodes.txt", "nodes setup file")
	dataDirFlag := logCommand.String("dataDir", "rrdData", "set rrd directory")

	logCommand.Parse(os.Args[0:])
	fmt.Println("interval is ", *intervalFlag, *durationFlag, *nodesFileFlag, *dataDirFlag)
	log(*intervalFlag, *durationFlag, *nodesFileFlag, *dataDirFlag)
}
