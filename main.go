package main

import (
	"fmt"
	"os"
	"tcp-lang-conn/mytcp"
)

func main() {
	if len(os.Args) <= 2 {
		fmt.Println("Usage: ./connTest [s|c] [ip_address:port]\n" +
			"Example: \n" +
			"1. Server(192.168.1.1) start:    ./connTest s 0.0.0.0:8001\n" +
			"2. Client(192.168.1.2) start:    ./connTest c 192.168.1.1:port")
	} else if len(os.Args) == 3 {
		mode, ip := os.Args[1], os.Args[2]
		switch mode {
			case "s": {
				mytcp.MyTcpServer(ip)
			}
			case "c": {
				mytcp.MyTcpClient(ip)
			}
		default:
			panic("First parameter error.")
		}
	} else {
		panic("Input parameter error.")
	}
	//os.Exit(1);
}
