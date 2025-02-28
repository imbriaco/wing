---
title: API reference
id: api-reference
description: Wing standard library API reference for the http module
keywords: [Wing sdk, sdk, Wing API Reference]
hide_title: true
sidebar_position: 100
---

<!-- This file is automatically generated. Do not edit manually. -->

# API Reference <a name="API Reference" id="api-reference"></a>


## Classes <a name="Classes" id="Classes"></a>

### Http <a name="Http" id="@winglang/sdk.http.Util"></a>

the Http class is used for calling different HTTP methods and requesting and sending information online,  as well as testing public accessible resources.


#### Static Functions <a name="Static Functions" id="Static Functions"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.http.Util.delete">delete</a></code> | Executes a DELETE request to a specified URL and provides a formatted response. |
| <code><a href="#@winglang/sdk.http.Util.fetch">fetch</a></code> | Executes a HTTP request to a specified URL and provides a formatted response. |
| <code><a href="#@winglang/sdk.http.Util.get">get</a></code> | Executes a GET request to a specified URL and provides a formatted response. |
| <code><a href="#@winglang/sdk.http.Util.patch">patch</a></code> | Executes a PATCH request to a specified URL and provides a formatted response. |
| <code><a href="#@winglang/sdk.http.Util.post">post</a></code> | Executes a POST request to a specified URL and provides a formatted response. |
| <code><a href="#@winglang/sdk.http.Util.put">put</a></code> | Executes a PUT request to a specified URL and provides a formatted response. |

---

##### `delete` <a name="delete" id="@winglang/sdk.http.Util.delete"></a>

```wing
bring http;

inflight http.delete(url: str, options?: RequestOptions);
```

Executes a DELETE request to a specified URL and provides a formatted response.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.delete.parameter.url"></a>

- *Type:* str

The target URL for the DELETE request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.delete.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

Optional parameters for customizing the DELETE request.

---

##### `fetch` <a name="fetch" id="@winglang/sdk.http.Util.fetch"></a>

```wing
bring http;

inflight http.fetch(url: str, options?: RequestOptions);
```

Executes a HTTP request to a specified URL and provides a formatted response.

This method allows various HTTP methods based on the provided options.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.fetch.parameter.url"></a>

- *Type:* str

The target URL for the request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.fetch.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

Optional parameters for customizing the HTTP request.

---

##### `get` <a name="get" id="@winglang/sdk.http.Util.get"></a>

```wing
bring http;

inflight http.get(url: str, options?: RequestOptions);
```

Executes a GET request to a specified URL and provides a formatted response.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.get.parameter.url"></a>

- *Type:* str

The target URL for the GET request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.get.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

Optional parameters for customizing the GET request.

---

##### `patch` <a name="patch" id="@winglang/sdk.http.Util.patch"></a>

```wing
bring http;

inflight http.patch(url: str, options?: RequestOptions);
```

Executes a PATCH request to a specified URL and provides a formatted response.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.patch.parameter.url"></a>

- *Type:* str

The target URL for the PATCH request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.patch.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

Optional parameters for customizing the PATCH request.

---

##### `post` <a name="post" id="@winglang/sdk.http.Util.post"></a>

```wing
bring http;

inflight http.post(url: str, options?: RequestOptions);
```

Executes a POST request to a specified URL and provides a formatted response.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.post.parameter.url"></a>

- *Type:* str

The target URL for the POST request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.post.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

Optional parameters for customizing the POST request.

---

##### `put` <a name="put" id="@winglang/sdk.http.Util.put"></a>

```wing
bring http;

inflight http.put(url: str, options?: RequestOptions);
```

Executes a PUT request to a specified URL and provides a formatted response.

###### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Util.put.parameter.url"></a>

- *Type:* str

The target URL for the PUT request.

---

###### `options`<sup>Optional</sup> <a name="options" id="@winglang/sdk.http.Util.put.parameter.options"></a>

- *Type:* <a href="#@winglang/sdk.http.RequestOptions">RequestOptions</a>

ptional parameters for customizing the PUT request.

---



## Structs <a name="Structs" id="Structs"></a>

### RequestOptions <a name="RequestOptions" id="@winglang/sdk.http.RequestOptions"></a>

An object containing any custom settings that you want to apply to the request.

#### Initializer <a name="Initializer" id="@winglang/sdk.http.RequestOptions.Initializer"></a>

```wing
bring http;

let RequestOptions = http.RequestOptions{ ... };
```

#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.body">body</a></code> | <code>str</code> | Any body that you want to add to your request. |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.cache">cache</a></code> | <code><a href="#@winglang/sdk.http.RequestCache">RequestCache</a></code> | The cache mode you want to use for the request. |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.headers">headers</a></code> | <code>MutMap&lt;str&gt;</code> | Any headers you want to add to your request. |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.method">method</a></code> | <code><a href="#@winglang/sdk.http.HttpMethod">HttpMethod</a></code> | The request method, e.g., GET, POST. The default is GET. |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.redirect">redirect</a></code> | <code><a href="#@winglang/sdk.http.RequestRedirect">RequestRedirect</a></code> | he redirect mode to use: follow, error. |
| <code><a href="#@winglang/sdk.http.RequestOptions.property.referrer">referrer</a></code> | <code>str</code> | A string specifying "no-referrer", client, or a URL. |

---

##### `body`<sup>Optional</sup> <a name="body" id="@winglang/sdk.http.RequestOptions.property.body"></a>

```wing
body: str;
```

- *Type:* str

Any body that you want to add to your request.

Note that a request using the GET or HEAD method cannot have a body.

---

##### `cache`<sup>Optional</sup> <a name="cache" id="@winglang/sdk.http.RequestOptions.property.cache"></a>

```wing
cache: RequestCache;
```

- *Type:* <a href="#@winglang/sdk.http.RequestCache">RequestCache</a>

The cache mode you want to use for the request.

---

##### `headers`<sup>Optional</sup> <a name="headers" id="@winglang/sdk.http.RequestOptions.property.headers"></a>

```wing
headers: MutMap<str>;
```

- *Type:* MutMap&lt;str&gt;

Any headers you want to add to your request.

---

##### `method`<sup>Optional</sup> <a name="method" id="@winglang/sdk.http.RequestOptions.property.method"></a>

```wing
method: HttpMethod;
```

- *Type:* <a href="#@winglang/sdk.http.HttpMethod">HttpMethod</a>
- *Default:* GET

The request method, e.g., GET, POST. The default is GET.

---

##### `redirect`<sup>Optional</sup> <a name="redirect" id="@winglang/sdk.http.RequestOptions.property.redirect"></a>

```wing
redirect: RequestRedirect;
```

- *Type:* <a href="#@winglang/sdk.http.RequestRedirect">RequestRedirect</a>
- *Default:* follow

he redirect mode to use: follow, error.

The default is follow.

---

##### `referrer`<sup>Optional</sup> <a name="referrer" id="@winglang/sdk.http.RequestOptions.property.referrer"></a>

```wing
referrer: str;
```

- *Type:* str
- *Default:* about:client

A string specifying "no-referrer", client, or a URL.

The default is "about:client".

---

### Response <a name="Response" id="@winglang/sdk.http.Response"></a>

The response to a HTTP request.

#### Initializer <a name="Initializer" id="@winglang/sdk.http.Response.Initializer"></a>

```wing
bring http;

let Response = http.Response{ ... };
```

#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.http.Response.property.headers">headers</a></code> | <code>MutMap&lt;str&gt;</code> | The map of header information associated with the response. |
| <code><a href="#@winglang/sdk.http.Response.property.ok">ok</a></code> | <code>bool</code> | A boolean indicating whether the response was successful (status in the range 200 – 299) or not. |
| <code><a href="#@winglang/sdk.http.Response.property.status">status</a></code> | <code>num</code> | The status code of the response. |
| <code><a href="#@winglang/sdk.http.Response.property.url">url</a></code> | <code>str</code> | The URL of the response. |
| <code><a href="#@winglang/sdk.http.Response.property.body">body</a></code> | <code>str</code> | A string representation of the body contents. |

---

##### `headers`<sup>Required</sup> <a name="headers" id="@winglang/sdk.http.Response.property.headers"></a>

```wing
headers: MutMap<str>;
```

- *Type:* MutMap&lt;str&gt;

The map of header information associated with the response.

---

##### `ok`<sup>Required</sup> <a name="ok" id="@winglang/sdk.http.Response.property.ok"></a>

```wing
ok: bool;
```

- *Type:* bool

A boolean indicating whether the response was successful (status in the range 200 – 299) or not.

---

##### `status`<sup>Required</sup> <a name="status" id="@winglang/sdk.http.Response.property.status"></a>

```wing
status: num;
```

- *Type:* num

The status code of the response.

(This will be 200 for a success).

---

##### `url`<sup>Required</sup> <a name="url" id="@winglang/sdk.http.Response.property.url"></a>

```wing
url: str;
```

- *Type:* str

The URL of the response.

---

##### `body`<sup>Optional</sup> <a name="body" id="@winglang/sdk.http.Response.property.body"></a>

```wing
body: str;
```

- *Type:* str

A string representation of the body contents.

---


## Enums <a name="Enums" id="Enums"></a>

### HttpMethod <a name="HttpMethod" id="@winglang/sdk.http.HttpMethod"></a>

The request's method.

#### Members <a name="Members" id="Members"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.http.HttpMethod.GET">GET</a></code> | GET. |
| <code><a href="#@winglang/sdk.http.HttpMethod.PUT">PUT</a></code> | PUT. |
| <code><a href="#@winglang/sdk.http.HttpMethod.DELETE">DELETE</a></code> | DELETE. |
| <code><a href="#@winglang/sdk.http.HttpMethod.PATCH">PATCH</a></code> | PATCH. |
| <code><a href="#@winglang/sdk.http.HttpMethod.POST">POST</a></code> | POST. |
| <code><a href="#@winglang/sdk.http.HttpMethod.OPTIONS">OPTIONS</a></code> | OPTIONS. |
| <code><a href="#@winglang/sdk.http.HttpMethod.HEAD">HEAD</a></code> | HEAD. |

---

##### `GET` <a name="GET" id="@winglang/sdk.http.HttpMethod.GET"></a>

GET.

---


##### `PUT` <a name="PUT" id="@winglang/sdk.http.HttpMethod.PUT"></a>

PUT.

---


##### `DELETE` <a name="DELETE" id="@winglang/sdk.http.HttpMethod.DELETE"></a>

DELETE.

---


##### `PATCH` <a name="PATCH" id="@winglang/sdk.http.HttpMethod.PATCH"></a>

PATCH.

---


##### `POST` <a name="POST" id="@winglang/sdk.http.HttpMethod.POST"></a>

POST.

---


##### `OPTIONS` <a name="OPTIONS" id="@winglang/sdk.http.HttpMethod.OPTIONS"></a>

OPTIONS.

---


##### `HEAD` <a name="HEAD" id="@winglang/sdk.http.HttpMethod.HEAD"></a>

HEAD.

---


### RequestCache <a name="RequestCache" id="@winglang/sdk.http.RequestCache"></a>

The cache mode of the request.

It controls how a request will interact with the system's HTTP cache.

#### Members <a name="Members" id="Members"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.http.RequestCache.DEFAULT">DEFAULT</a></code> | The runtime environment looks for a matching request in its HTTP cache. |
| <code><a href="#@winglang/sdk.http.RequestCache.NO_STORE">NO_STORE</a></code> | The runtime environment fetches the resource from the remote server without first looking in the cache, and will not update the cache with the downloaded resource. |
| <code><a href="#@winglang/sdk.http.RequestCache.RELOAD">RELOAD</a></code> | The runtime environment fetches the resource from the remote server without first looking in the cache, but then will update the cache with the downloaded resource. |
| <code><a href="#@winglang/sdk.http.RequestCache.NO_CACHE">NO_CACHE</a></code> | The runtime environment looks for a matching request in its HTTP cache. |
| <code><a href="#@winglang/sdk.http.RequestCache.FORCE_CACHE">FORCE_CACHE</a></code> | The runtime environment looks for a matching request in its HTTP cache. |

---

##### `DEFAULT` <a name="DEFAULT" id="@winglang/sdk.http.RequestCache.DEFAULT"></a>

The runtime environment looks for a matching request in its HTTP cache.

* If there is a match and it is fresh, it will be returned from the cache.
* If there is a match but it is stale, the runtime environment will make a conditional request to the remote server.
* If the server indicates that the resource has not changed, it will be returned from the cache.
* Otherwise the resource will be downloaded from the server and the cache will be updated.
* If there is no match, the runtime environment will make a normal request, and will update the cache with the downloaded resource.

---


##### `NO_STORE` <a name="NO_STORE" id="@winglang/sdk.http.RequestCache.NO_STORE"></a>

The runtime environment fetches the resource from the remote server without first looking in the cache, and will not update the cache with the downloaded resource.

---


##### `RELOAD` <a name="RELOAD" id="@winglang/sdk.http.RequestCache.RELOAD"></a>

The runtime environment fetches the resource from the remote server without first looking in the cache, but then will update the cache with the downloaded resource.

---


##### `NO_CACHE` <a name="NO_CACHE" id="@winglang/sdk.http.RequestCache.NO_CACHE"></a>

The runtime environment looks for a matching request in its HTTP cache.

* If there is a match, fresh or stale, the runtime environment will make a conditional request to the remote server.
* If the server indicates that the resource has not changed, it will be returned from the cache. Otherwise the resource will be downloaded from the server and the cache will be updated.
* If there is no match, the runtime environment will make a normal request, and will update the cache with the downloaded resource.

---


##### `FORCE_CACHE` <a name="FORCE_CACHE" id="@winglang/sdk.http.RequestCache.FORCE_CACHE"></a>

The runtime environment looks for a matching request in its HTTP cache.

* If there is a match, fresh or stale, it will be returned from the cache.
* If there is no match, the runtime environment will make a normal request, and will update the cache with the downloaded resource.

---


### RequestRedirect <a name="RequestRedirect" id="@winglang/sdk.http.RequestRedirect"></a>

The redirect read-only property that contains the mode for how redirects are handled.

#### Members <a name="Members" id="Members"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.http.RequestRedirect.FOLLOW">FOLLOW</a></code> | Follow all redirects incurred when fetching a resource. |
| <code><a href="#@winglang/sdk.http.RequestRedirect.ERROR">ERROR</a></code> | Return a network error when a request is met with a redirect. |

---

##### `FOLLOW` <a name="FOLLOW" id="@winglang/sdk.http.RequestRedirect.FOLLOW"></a>

Follow all redirects incurred when fetching a resource.

---


##### `ERROR` <a name="ERROR" id="@winglang/sdk.http.RequestRedirect.ERROR"></a>

Return a network error when a request is met with a redirect.

---

