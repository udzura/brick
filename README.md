# brick

Remote/async code executor for sandbox environment

## Run

```console
$ cargo run
```

## Usage

```console
$ curl -sd '{"cmd": "for i in {1..10}; do echo $i; sleep 3; done"}' localhost:8000/enqueue | \
> jq .
{
  "status": 200,
  "id": 3482604423,
  "msg": "Enqueued"
}

$ curl -s localhost:8000/status/3482604423 | jq .stdout -r
1
2
3
4
...

# waiting for command done
$ curl -s localhost:8000/status/3482604423 | jq .
{
  "status": 200,
  "id": 3482604423,
  "running": false,
  "cmd_result": 0,
  "stdout": "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n",
  "msg": null
}
```
