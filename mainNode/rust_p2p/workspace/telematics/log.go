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
	"sort"
)

type Snapshot struct {
	Generated_transactions				int
	Confirmed_transactions				int
	Chain_depth										int
	Token													bool
	Propose_block									int
	Sign_block										int
	Submit_block									int
	Propose_latency								int
	Sign_latency									int
	Submit_latency								int
	Gas														int
	Propose_num										int
	Sign_num											int
	Submit_num										int
}

type Report struct {
	Node string
	Data Snapshot
}

func log(interval, duration uint, nodesFile, dataDir, logDir string) {
	fmt.Println("scan nodesFile ", nodesFile)
	nodes := make(map[string]string)
	file, err := os.Open(nodesFile)
	if err != nil {
		fmt.Println("Error nodes config file is empty")
		os.Exit(1)
	}
	defer file.Close()

	scanner := bufio.NewScanner(file)
	var num_scale = 0
	var num_side = 0
	for scanner.Scan() {
		line := scanner.Text()
		if line[0] == '#' {
			continue
		}
		tokens := strings.Split(line, ",")
		name := tokens[0]
		//id := tokens[1]
		ip := tokens[2]
		//p2p_port := tokens[3]
		api_port := tokens[4]
		scale_id := tokens[5]
		if scale_id == "0" {
			num_side += 1
		} else {
			num_scale += 1
		}
		url := fmt.Sprintf("http://%v:%v/telematics/snapshot", ip, api_port)
		nodes[name] = url
	}
  if scanner.Err() != nil {
		fmt.Println("Error scan nodesFile")
		os.Exit(1)
	}

	fmt.Println("num side %v. num scale %v", num_side, num_scale)

	fmt.Println("Create rrd database")
	err = os.MkdirAll(dataDir, os.ModeDir | os.FileMode(0755))
	if err != nil {
		fmt.Println("Error create rrd dir")
		os.Exit(1)
	}

	for node, _ := range nodes {
		p := path.Clean(dataDir + "/" + node + ".rrd")
		fmt.Println(p)
		c := rrd.NewCreator(p, time.Now(), interval)
		c.DS("generated_tx", "COUNTER", interval*2, 0, "U")
		c.DS("confirmed_tx", "COUNTER", interval*2, 0, "U")
		c.DS("chain_depth", "GAUGE", interval*2, 0, "U")
		c.DS("propose_latency", "COUNTER", interval*2, 0, "U")
		c.DS("sign_latency", "COUNTER", interval*2, 0, "U")
		c.DS("submit_latency", "COUNTER", interval*2, 0, "U")
		c.DS("gas", "COUNTER", interval*2, 0, "U")
		c.DS("propose_num", "GAUGE", interval*2, 0, "U")
		c.DS("sign_num", "GAUGE", interval*2, 0, "U")
		c.DS("submit_num", "GAUGE", interval*2, 0, "U")
		c.DS("propose_delay_mean", "COMPUTE", "propose_latency,propose_num,/")
		c.DS("sign_delay_mean", "COMPUTE", "sign_latency,sign_num,/")
		c.DS("submit_delay_mean", "COMPUTE", "submit_latency,submit_num,/")
		c.RRA("LAST", 0, 1, duration)
		err = c.Create(true)
		if err != nil {
			fmt.Println("Error create rrd: ", err)
			os.Exit(1)
		}
	}

	//fmt.Println(nodes)

	fmt.Println("start routine to collect rrd data point")
	c := make(chan Report)
	for node, url := range nodes {
		monitor(url, node, dataDir, interval, c)
	}

	fmt.Println("present and display")
	prev := make(map[string]Snapshot)
	curr := make(map[string]Snapshot)
	logWriter := make(map[string] *bufio.Writer)
	ticker := time.NewTicker(time.Duration(interval) * time.Second)
	start := time.Now()
	snapshot_counter := 0
	keys := make([]string, len(nodes))
	for k := range nodes {
			keys = append(keys, k)
	}
	sort.Strings(keys)
	for k := range nodes {
		filename := logDir + "/" + k + ".txt"
		f, err := os.Create(filename)
		w := bufio.NewWriter(f)
		defer f.Close()
		fmt.Println(filename)
		if err != nil {
			fmt.Println("Error create log.txt", err)
		}
		logWriter[k] = w
	}

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
				fmt.Println("curr:", curr)
				sum_over_nodes(&curr, &csum)
				sum_over_nodes(&prev, &psum)

				comp_avg(&csum, &cavg, num_scale, num_side)
				comp_avg(&psum, &pavg, num_scale, num_side)

				for k := range nodes {
					var token = 0
					if curr[k].Token {
						token = 1
					}

					writer := logWriter[k]
					curr_time := time.Now().Unix()
					content := fmt.Sprintf("%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,%v,\n", curr_time, curr[k].Generated_transactions,
						curr[k].Confirmed_transactions,
						curr[k].Chain_depth,
						token,
						curr[k].Propose_block,
						curr[k].Sign_block,
						curr[k].Submit_block,
						curr[k].Propose_latency,
						curr[k].Sign_latency,
						curr[k].Submit_latency,
						curr[k].Gas,
						curr[k].Propose_num,
						curr[k].Sign_num,
						curr[k].Submit_num)
					_, err := writer.WriteString(content)
					writer.Flush()
					if err != nil {
							panic(err)
					}
				}

				// present data
				dur := now.Sub(start).Seconds()
				tm.Clear()
				tm.MoveCursor(1,1)
				tm.Printf("Telemetics durateion : %v sec\n", int(dur))
				tm.Printf("                            %v    \n", "Overall")
				tm.Printf("Generated transaction  :    %8.1f\n", float64(cavg.Generated_transactions)/dur) //, float64(cavg.Generated_transactions-pavg.Generated_transactions)/float64(interval)
				tm.Printf("Confirmed transaction  :    %8.1f\n", float64(cavg.Confirmed_transactions)/dur) //, float64(cavg.Confirmed_transactions-pavg.Confirmed_transactions)/float64(interval)
				tm.Printf("Chain depth            :    %8v\n", cavg.Chain_depth) //, float64(cavg.Chain_depth-pavg.Chain_depth)/float64(interval)
				tm.Printf("Gas                    :    %8v\n", csum.Gas) //, float64(csum.Gas-psum.Gas)/float64(interval)
				tm.Printf("latency - propose      :    %8v\n", float64(cavg.Propose_latency)/float64(cavg.Chain_depth+1)/1000.0) //, float64(cavg.Propose_latency-pavg.Propose_latency)/float64(cavg.Chain_depth-pavg.Chain_depth)/1000.0
				tm.Printf("latency - sign         :    %8v\n", float64(cavg.Sign_latency)/float64(cavg.Chain_depth+1)/1000.0) //, float64(cavg.Sign_latency-pavg.Sign_latency)/float64(cavg.Chain_depth-pavg.Chain_depth)/1000.0
				tm.Printf("latency - submit       :    %8v\n", float64(cavg.Submit_latency)/float64(cavg.Chain_depth+1)/1000.0) //, float64(cavg.Submit_latency-pavg.Submit_latency)/float64(cavg.Chain_depth-pavg.Chain_depth)/1000.0
				tm.Printf("\n")
				tm.Printf("Nodes         Depth			Propose(id)     	Sign(id)       Submit(id)\n")
				//fmt.Println("keys:", keys)
				//os.Exit(0)

				for _, node := range keys {
					ns, ok := curr[node]
					if ok {
						var token = " "
						if ns.Token {
							token = "*"
						}
						tm.Printf("%v(%v):         %v			%v		%v		%v\n", node, token,  ns.Chain_depth, ns.Propose_block, ns.Sign_block, ns.Submit_block)
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
		sumshot.Confirmed_transactions += v.Confirmed_transactions
		sumshot.Chain_depth += v.Chain_depth
		sumshot.Propose_latency += v.Propose_latency
		sumshot.Sign_latency += v.Sign_latency
		sumshot.Submit_latency += v.Submit_latency
		sumshot.Gas += v.Gas
	}
}

func comp_avg(tot *Snapshot, avg *Snapshot, num_scale, num_side int){
	avg.Generated_transactions = tot.Generated_transactions/num_side
	avg.Confirmed_transactions = tot.Confirmed_transactions/num_side
	avg.Chain_depth = tot.Chain_depth/num_side
	avg.Propose_latency = tot.Propose_latency
	avg.Sign_latency = tot.Sign_latency/num_scale
	avg.Submit_latency = tot.Submit_latency/num_scale
	avg.Gas = tot.Gas/num_scale
}


func monitor(url, node, dataDir string, interval uint, c chan Report) {
	ticker := time.NewTicker(time.Duration(interval) * time.Second)
	updater := rrd.NewUpdater(dataDir + "/" + node + ".rrd")
	go func() {
		for range ticker.C {
			resp, err := http.Get(url)
			if err != nil {
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

			err = updater.Update(time.Now(), snapshot.Generated_transactions, snapshot.Confirmed_transactions, snapshot.Chain_depth, snapshot.Propose_latency, snapshot.Sign_latency, snapshot.Submit_latency, snapshot.Gas, snapshot.Propose_num, snapshot.Sign_num, snapshot.Submit_num)
			if err != nil {
				fmt.Println("rrd updater err:", err)
				continue
			}
			c <- Report {node, snapshot}
		}
	}()
}
