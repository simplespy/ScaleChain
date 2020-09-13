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
	nodesFileFlag := logCommand.String("nodesFileFlag", "./nodes.txt", "nodes setup file")
	dataDirFlag := logCommand.String("dataDir", "./rrdData", "set rrd directory")
	logDirFlag := logCommand.String("logDir", "./logData", "set csv log directory")

	var plotCommand = flag.NewFlagSet("plot", flag.ExitOnError)
	plotNodeListFlag := plotCommand.String("nodelist", "../nodes.txt", "Sets the path to the node list file")
  plotNodeFlag := plotCommand.String("node", "node_1", "Sets the node to plot")
	plotContentFlag := plotCommand.String("content", "block", "Sets the content to plot, possible values are txrate, blockdelay, queue, mining, confirm")
	plotDataDirFlag := plotCommand.String("dataDir", "./rrdData", "Sets the path to the directory holding RRD files")
	plotOutputFlag := plotCommand.String("output", "output.png", "Sets the output path")
	plotStartFlag := plotCommand.Int64("start", 1, "start time in Unix epoch")
  plotDurationFlag := plotCommand.Int64("duration", 600, "Sets the time span for the plot")

	if len(os.Args) < 2 {
		fmt.Println("Subcommand: log, plot")
		os.Exit(1)
	}
	switch os.Args[1] {
	case "log":
		logCommand.Parse(os.Args[2:])
		fmt.Println("interval is ", *intervalFlag, *durationFlag, *nodesFileFlag, *dataDirFlag)
		log(*intervalFlag, *durationFlag, *nodesFileFlag, *dataDirFlag, *logDirFlag)
	case "plot":
		plotCommand.Parse(os.Args[2:])
		plot(*plotNodeListFlag, *plotNodeFlag, *plotContentFlag, *plotDataDirFlag, *plotOutputFlag, *plotStartFlag, *plotDurationFlag)
	default:
		fmt.Println("Subcommands: log, plot")
		os.Exit(1)
	}
}
