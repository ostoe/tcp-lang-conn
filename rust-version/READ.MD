# fw测试
input file:
```
10.0.0.1-10.0.1.8,192.168.1.1/24  icmp,tcp:[8080, 1080-1090, 7654]
...
```
out put:
```
service health port
ip | port
----
can access port
ip | port

---
can't access port
ip | port

```