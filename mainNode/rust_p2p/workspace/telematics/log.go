package main

import (
	"fmt"
	"bufio"
	"encoding/json"
	tm "github.com/buger/goterm"
	"github.com/ziutek/rrd"
	"net/http"
	"os"
	"path"
	"strings"
	"time"
)

type Snapshot struct {
	Generated_transactions				int
	Chain_depth										int
}

type Report struct {
	Node string
	Data Snapshot
}

func log(interval, duration uint, nodesFile, dataDir string) {
	fmt.Println("scan nodesFile")
	nodes := make(map[string]string)
	file, err := os.Open(nodesFile)
	if err != nil {
		fmt.Println("Error nodes config file is empty")
		os.Exit(1)
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	for scanner.Scan() {
		line := scanner.Text()
		if line[0] == '#' {
			continue
		}
		tokens := strings.Split(line, ",")
		name := tokens[0]
		ip := tokens[1]
		port := tokens[2]
		url := fmt.Sprintf("http://%v:%v/telematics/snapshot", ip, port)
		nodes[name] = url
	}
  if scanner.Err() != nil {
		fmt.Println("Error scan nodesFile")
		os.Exit(1)
	}

	fmt.Println("Create rrd database")
	err = os.MkdirAll(dataDir, os.ModeDir | os.FileMode(0755))
	if err != nil {
		fmt.Println("Error create rrd dir")
		os.Exit(1)
	}

	for node, _ := range nodes {
		p := path.Clean(dataDir + "/" + node + ".rdd")
		fmt.Println(p)
		c := rrd.NewCreator(p, time.Now(), interval)
		c.DS("genrated_tx", "COUNTER", interval*2, 0, "U")
		c.DS("chain_depth", "COUNTER", interval*2, 0, "U")
		c.RRA("LAST", 0, 1, duration)
		err = c.Create(true)
		if err != nil {
			fmt.Println("Error create rrd: ", err)
			os.Exit(1)
		}
	}

	fmt.Println(nodes)

	fmt.Println("start routine to collect rrd data point")
	c := make(chan Report)
	for node, url := range nodes {
		monitor(url, node, dataDir, interval, c)
	}

	fmt.Println("present and display")
	prev := make(map[string]Snapshot)
	curr := make(map[string]Snapshot)
	ticker := time.NewTicker(time.Duration(interval) * time.Second)
	start := time.Now()
	snapshot_counter := 0

	go func() {
		for {
			select {
			case report := <-c:
				// update snapshot
				snapshot_counter += 1
				cp, ok := curr[report.Node]
				if ok {
					prev[report.Node] = cp
				}
				curr[report.Node] = report.Data
			case now := <-ticker.C:
				// process and display the result, sum over all nodes
				if len(curr) == 0 || len(prev) == 0 {
					continue
				}
				csum, cavg := Snapshot{}, Snapshot{}
				psum, pavg := Snapshot{}, Snapshot{}

				sum_over_nodes(&curr, &csum)
				sum_over_nodes(&prev, &psum)

				comp_avg(&csum, &cavg, len(curr))
				comp_avg(&psum, &pavg, len(prev))

				// present data
				dur := now.Sub(start).Seconds()
				tm.Clear()
				tm.MoveCursor(1,1)
				tm.Printf("Telemetics durateion : %v sec\n", int(dur))
				tm.Printf("                             %v    %v\n", "Overall", "Window") 
				tm.Printf("Generated transaction  :    %8.1f  %8.1f\n", float64(cavg.Generated_transactions)/dur, float64(cavg.Generated_transactions-pavg.Generated_transactions)/float64(interval))
				tm.Printf("Chain depth            :    %8v  %8g\n", cavg.Chain_depth, float64(cavg.Chain_depth-pavg.Chain_depth)/float64(interval))
				tm.Printf("\n")
				tm.Printf("Nodes         Depth\n")
				for node, _ := range nodes {
					node_snapshot, ok := curr[node]
					if ok {
						tm.Printf("node %v:          %v\n", node, node_snapshot.Chain_depth)
					}
				}
				tm.Flush()
			}
		}
	}()

	select{}
}


func sum_over_nodes(curr *map[string]Snapshot, sumshot *Snapshot) {
	for _, v := range *curr {
		sumshot.Generated_transactions += v.Generated_transactions
		sumshot.Chain_depth += v.Chain_depth
	}
}

func comp_avg(tot *Snapshot, avg *Snapshot, num_node int){
	avg.Generated_transactions = tot.Generated_transactions/num_node
	avg.Chain_depth = tot.Chain_depth/num_node
}


func monitor(url, node, dataDir string, interval uint, c chan Report) {
	ticker := time.NewTicker(time.Duration(interval) * time.Second)
	p := path.Clean(dataDir + "/" + node + ".rdd")
	updater := rrd.NewUpdater(p)
	go func() {
		for range ticker.C {
			resp, err := http.Get(url)
			if err != nil {
				//fmt.Println("Error open http connection:", err)
				continue
			}
			defer resp.Body.Close()

			decoder := json.NewDecoder(resp.Body)
			snapshot := Snapshot{}
			err = decoder.Decode(&snapshot)
			if err != nil {
				fmt.Println("Error read json:", err)
				continue
			}

			err = updater.Update(time.Now(), snapshot.Generated_transactions, snapshot.Chain_depth)
			if err != nil {
				fmt.Println("rrd updater err:", err)
				continue
			}
			c <- Report {node, snapshot}
		}
	}()
}
