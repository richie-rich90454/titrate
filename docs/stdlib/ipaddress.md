# ipaddress

The `tt.net` module provides `IpAddress` — a class for inspecting and classifying IP addresses (both IPv4 and IPv6).

```titrate
import tt.net.IpAddress;
```

## IpAddress

Represents an IP address and provides classification methods to determine its type and scope.

- `fn init(address: string)` — create an IpAddress from a string (IPv4 or IPv6)
- `isLoopback(): bool` — check if the address is a loopback address
- `isPrivate(): bool` — check if the address is a private/reserved-for-private-use address
- `isMulticast(): bool` — check if the address is a multicast address
- `isReserved(): bool` — check if the address is reserved by IANA
- `isLinkLocal(): bool` — check if the address is link-local
- `isGlobal(): bool` — check if the address is globally routable
- `isV4(): bool` — check if the address is IPv4
- `isV6(): bool` — check if the address is IPv6
- `toString(): string` — return the string representation of the address
- `equals(other: IpAddress): bool` — check equality against another IpAddress

```titrate
let local: IpAddress = new IpAddress("127.0.0.1");
io::println(Boolean.toString(local.isLoopback())); // true
io::println(Boolean.toString(local.isV4()));        // true

let privateAddr: IpAddress = new IpAddress("192.168.1.1");
io::println(Boolean.toString(privateAddr.isPrivate())); // true
io::println(Boolean.toString(privateAddr.isGlobal()));  // false

let v6: IpAddress = new IpAddress("::1");
io::println(Boolean.toString(v6.isV6()));       // true
io::println(Boolean.toString(v6.isLoopback())); // true
```

## Free Functions

- `ipv4(address: string): IpAddress` — create an IpAddress explicitly as IPv4
- `ipv6(address: string): IpAddress` — create an IpAddress explicitly as IPv6
- `isV4(address: string): bool` — check if a string is a valid IPv4 address
- `isV6(address: string): bool` — check if a string is a valid IPv6 address
- `validate(address: string): bool` — check if a string is a valid IP address (either v4 or v6)

```titrate
io::println(Boolean.toString(isV4("10.0.0.1")));     // true
io::println(Boolean.toString(isV6("fe80::1")));       // true
io::println(Boolean.toString(validate("not an ip"))); // false

let addr: IpAddress = ipv4("8.8.8.8");
io::println(Boolean.toString(addr.isGlobal())); // true
```
