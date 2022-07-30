package main

import (
	"fmt"
	"os"
	"strconv"
	"tcp-lang-conn/mytcp"
)

func main() {
	if len(os.Args) <= 2 {
		fmt.Println("Usage: ./connTest [s|c] [ip_address:port]\n" +
			"Example: \n" +
			"1. Server(192.168.1.1) start:    ./connTest s 0.0.0.0:8001\n" +
			"2. Client(192.168.1.2) start:    ./connTest c 192.168.1.1:8001 [|optional t minute] t 30")
	} else if len(os.Args) == 3 {
		mode, ip := os.Args[1], os.Args[2]
		switch mode {
			case "s": {
				mytcp.MyTcpServer(ip)
			}
			case "c": {
				mytcp.MyTcpClient(ip, 30)
			}
		default:
			panic("First parameter error.")
		}
	} else if len(os.Args) == 5 {
		mode, ip, tmode, t := os.Args[1], os.Args[2], os.Args[3], os.Args[4]
		if mode == "c" && tmode == "t" {
			tint, err := strconv.Atoi(t)
			if err != nil {panic("Input timeout parameter error.")}
			mytcp.MyTcpClient(ip, tint)
		} else {
			panic("Input parameter error.")
		}
	} else {
		panic("Input parameter error.")
	}
	//os.Exit(1);
}
