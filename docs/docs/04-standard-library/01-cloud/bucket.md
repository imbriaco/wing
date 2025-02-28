---
title: Bucket
id: bucket
description: A built-in resource for handling object storage in the cloud.
keywords:
  [
    Wing reference,
    Wing language,
    language,
    Wing standard library,
    Wing programming language,
    Object storage,
    Buckets,
  ]
sidebar_position: 1
---

The `cloud.Bucket` resource represents a container for storing data in the cloud.

Buckets are a common way to store arbitrary files (images, videos, etc.), but can also be used to store structured data like JSON or CSV files.

Buckets in the cloud use object storage, which is optimized for storing large amounts of data with high availability.
Unlike other kinds of storage like file storage, data is not stored in a hierarchical structure, but rather as a flat list of objects, each associated with a key.

## Usage

### Defining a bucket

```js
bring cloud;

let bucket = new cloud.Bucket(
  public: true, // optional, defaults to `false`
);
```

### Populating objects during deployment

If you have static data that you want to upload to the bucket each time your app is deployed, you can call the preflight method `addObject`:

```js
bring cloud;

let bucket = new cloud.Bucket();

bucket.addObject("my-file.txt", "Hello, world!");
```

### Using a bucket inflight

```js playground
bring cloud;

let bucket = new cloud.Bucket();

let bucketFunc = inflight () => {
  bucket.put("file.txt", "Hello, world!");
  bucket.putJson("person.json", Json { name: "Alice" });

  let fileData = bucket.get("file.txt");
  assert(fileData == "Hello, world!");

  let jsonData = bucket.getJson("person.json");
  assert(jsonData.get("name") == "Alice");

  let keys = bucket.list();
  assert(keys.at(0) == "file.txt");
  assert(keys.at(1) == "person.json");

  bucket.delete("file.txt");
};

new cloud.Function(bucketFunc);
```

### Run code on bucket events

You can use the preflight methods `onCreate`, `onUpdate`, and `onDelete` to define code that should run when an object is uploaded, updated, or removed from the bucket.
Use the `onEvent` method for responding to any event.

Each method creates a new `cloud.Function` resource which will be triggered by the given event type.

```js playground
bring cloud;

let store = new cloud.Bucket();
let copies = new cloud.Bucket() as "Backup";

store.onCreate(inflight (key: str) => {
  let data = store.get(key);
  if !key.endsWith(".log") {
    copies.put(key, data);
  }
});

store.onDelete(inflight (key: str) => {
  copies.delete(key);
  log("Deleted ${key}");
});
```

## Target-specific details

### Simulator (`sim`)

Under the hood, the simulator uses a temporary local directory to store bucket data.

Note that bucket data is not persisted between simulator runs.

### AWS (`tf-aws` and `awscdk`)

The AWS implementation of `cloud.Bucket` uses [Amazon S3](https://aws.amazon.com/s3/).

### Azure (`tf-azure`)

The Azure implementation of `cloud.Bucket` uses [Azure Blob Storage](https://learn.microsoft.com/en-us/azure/storage/blobs/storage-blobs-overview).

### GCP (`tf-gcp`)

The Google Cloud implementation of `cloud.Bucket` uses [Google Cloud Storage](https://cloud.google.com/storage).
## API Reference <a name="API Reference" id="API Reference"></a>

### Bucket <a name="Bucket" id="@winglang/sdk.cloud.Bucket"></a>

A cloud object store.

#### Initializers <a name="Initializers" id="@winglang/sdk.cloud.Bucket.Initializer"></a>

```wing
bring cloud;

new cloud.Bucket(props?: BucketProps);
```

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.Bucket.Initializer.parameter.props">props</a></code> | <code><a href="#@winglang/sdk.cloud.BucketProps">BucketProps</a></code> | *No description.* |

---

##### `props`<sup>Optional</sup> <a name="props" id="@winglang/sdk.cloud.Bucket.Initializer.parameter.props"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketProps">BucketProps</a>

---

#### Methods <a name="Methods" id="Methods"></a>

##### Preflight Methods

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.cloud.Bucket.addObject">addObject</a></code> | Add a file to the bucket that is uploaded when the app is deployed. |
| <code><a href="#@winglang/sdk.cloud.Bucket.onCreate">onCreate</a></code> | Run an inflight whenever a file is uploaded to the bucket. |
| <code><a href="#@winglang/sdk.cloud.Bucket.onDelete">onDelete</a></code> | Run an inflight whenever a file is deleted from the bucket. |
| <code><a href="#@winglang/sdk.cloud.Bucket.onEvent">onEvent</a></code> | Run an inflight whenever a file is uploaded, modified, or deleted from the bucket. |
| <code><a href="#@winglang/sdk.cloud.Bucket.onUpdate">onUpdate</a></code> | Run an inflight whenever a file is updated in the bucket. |

##### Inflight Methods

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.delete">delete</a></code> | Delete an existing object using a key from the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.exists">exists</a></code> | Check if an object exists in the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.get">get</a></code> | Retrieve an object from the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.getJson">getJson</a></code> | Retrieve a Json object from the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.list">list</a></code> | Retrieve existing objects keys from the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.publicUrl">publicUrl</a></code> | Returns a url to the given file. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.put">put</a></code> | Put an object in the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.putJson">putJson</a></code> | Put a Json object in the bucket. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.tryDelete">tryDelete</a></code> | Delete an object from the bucket if it exists. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.tryGet">tryGet</a></code> | Get an object from the bucket if it exists. |
| <code><a href="#@winglang/sdk.cloud.IBucketClient.tryGetJson">tryGetJson</a></code> | Gets an object from the bucket if it exists, parsing it as Json. |

---

##### `addObject` <a name="addObject" id="@winglang/sdk.cloud.Bucket.addObject"></a>

```wing
addObject(key: str, body: str): void
```

Add a file to the bucket that is uploaded when the app is deployed.

TODO: In the future this will support uploading any `Blob` type or
referencing a file from the local filesystem.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.Bucket.addObject.parameter.key"></a>

- *Type:* str

---

###### `body`<sup>Required</sup> <a name="body" id="@winglang/sdk.cloud.Bucket.addObject.parameter.body"></a>

- *Type:* str

---

##### `onCreate` <a name="onCreate" id="@winglang/sdk.cloud.Bucket.onCreate"></a>

```wing
onCreate(fn: IBucketEventHandler, opts?: BucketOnCreateProps): void
```

Run an inflight whenever a file is uploaded to the bucket.

###### `fn`<sup>Required</sup> <a name="fn" id="@winglang/sdk.cloud.Bucket.onCreate.parameter.fn"></a>

- *Type:* <a href="#@winglang/sdk.cloud.IBucketEventHandler">IBucketEventHandler</a>

---

###### `opts`<sup>Optional</sup> <a name="opts" id="@winglang/sdk.cloud.Bucket.onCreate.parameter.opts"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketOnCreateProps">BucketOnCreateProps</a>

---

##### `onDelete` <a name="onDelete" id="@winglang/sdk.cloud.Bucket.onDelete"></a>

```wing
onDelete(fn: IBucketEventHandler, opts?: BucketOnDeleteProps): void
```

Run an inflight whenever a file is deleted from the bucket.

###### `fn`<sup>Required</sup> <a name="fn" id="@winglang/sdk.cloud.Bucket.onDelete.parameter.fn"></a>

- *Type:* <a href="#@winglang/sdk.cloud.IBucketEventHandler">IBucketEventHandler</a>

---

###### `opts`<sup>Optional</sup> <a name="opts" id="@winglang/sdk.cloud.Bucket.onDelete.parameter.opts"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketOnDeleteProps">BucketOnDeleteProps</a>

---

##### `onEvent` <a name="onEvent" id="@winglang/sdk.cloud.Bucket.onEvent"></a>

```wing
onEvent(fn: IBucketEventHandler, opts?: BucketOnEventProps): void
```

Run an inflight whenever a file is uploaded, modified, or deleted from the bucket.

###### `fn`<sup>Required</sup> <a name="fn" id="@winglang/sdk.cloud.Bucket.onEvent.parameter.fn"></a>

- *Type:* <a href="#@winglang/sdk.cloud.IBucketEventHandler">IBucketEventHandler</a>

---

###### `opts`<sup>Optional</sup> <a name="opts" id="@winglang/sdk.cloud.Bucket.onEvent.parameter.opts"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketOnEventProps">BucketOnEventProps</a>

---

##### `onUpdate` <a name="onUpdate" id="@winglang/sdk.cloud.Bucket.onUpdate"></a>

```wing
onUpdate(fn: IBucketEventHandler, opts?: BucketOnUpdateProps): void
```

Run an inflight whenever a file is updated in the bucket.

###### `fn`<sup>Required</sup> <a name="fn" id="@winglang/sdk.cloud.Bucket.onUpdate.parameter.fn"></a>

- *Type:* <a href="#@winglang/sdk.cloud.IBucketEventHandler">IBucketEventHandler</a>

---

###### `opts`<sup>Optional</sup> <a name="opts" id="@winglang/sdk.cloud.Bucket.onUpdate.parameter.opts"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketOnUpdateProps">BucketOnUpdateProps</a>

---

##### `delete` <a name="delete" id="@winglang/sdk.cloud.IBucketClient.delete"></a>

```wing
inflight delete(key: str, opts?: BucketDeleteOptions): void
```

Delete an existing object using a key from the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.delete.parameter.key"></a>

- *Type:* str

Key of the object.

---

###### `opts`<sup>Optional</sup> <a name="opts" id="@winglang/sdk.cloud.IBucketClient.delete.parameter.opts"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketDeleteOptions">BucketDeleteOptions</a>

Options available for delete an item from a bucket.

---

##### `exists` <a name="exists" id="@winglang/sdk.cloud.IBucketClient.exists"></a>

```wing
inflight exists(key: str): bool
```

Check if an object exists in the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.exists.parameter.key"></a>

- *Type:* str

Key of the object.

---

##### `get` <a name="get" id="@winglang/sdk.cloud.IBucketClient.get"></a>

```wing
inflight get(key: str): str
```

Retrieve an object from the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.get.parameter.key"></a>

- *Type:* str

Key of the object.

---

##### `getJson` <a name="getJson" id="@winglang/sdk.cloud.IBucketClient.getJson"></a>

```wing
inflight getJson(key: str): Json
```

Retrieve a Json object from the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.getJson.parameter.key"></a>

- *Type:* str

Key of the object.

---

##### `list` <a name="list" id="@winglang/sdk.cloud.IBucketClient.list"></a>

```wing
inflight list(prefix?: str): MutArray<str>
```

Retrieve existing objects keys from the bucket.

###### `prefix`<sup>Optional</sup> <a name="prefix" id="@winglang/sdk.cloud.IBucketClient.list.parameter.prefix"></a>

- *Type:* str

Limits the response to keys that begin with the specified prefix.

---

##### `publicUrl` <a name="publicUrl" id="@winglang/sdk.cloud.IBucketClient.publicUrl"></a>

```wing
inflight publicUrl(key: str): str
```

Returns a url to the given file.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.publicUrl.parameter.key"></a>

- *Type:* str

---

##### `put` <a name="put" id="@winglang/sdk.cloud.IBucketClient.put"></a>

```wing
inflight put(key: str, body: str): void
```

Put an object in the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.put.parameter.key"></a>

- *Type:* str

Key of the object.

---

###### `body`<sup>Required</sup> <a name="body" id="@winglang/sdk.cloud.IBucketClient.put.parameter.body"></a>

- *Type:* str

Content of the object we want to store into the bucket.

---

##### `putJson` <a name="putJson" id="@winglang/sdk.cloud.IBucketClient.putJson"></a>

```wing
inflight putJson(key: str, body: Json): void
```

Put a Json object in the bucket.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.putJson.parameter.key"></a>

- *Type:* str

Key of the object.

---

###### `body`<sup>Required</sup> <a name="body" id="@winglang/sdk.cloud.IBucketClient.putJson.parameter.body"></a>

- *Type:* <a href="#@winglang/sdk.std.Json">Json</a>

Json object that we want to store into the bucket.

---

##### `tryDelete` <a name="tryDelete" id="@winglang/sdk.cloud.IBucketClient.tryDelete"></a>

```wing
inflight tryDelete(key: str): bool
```

Delete an object from the bucket if it exists.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.tryDelete.parameter.key"></a>

- *Type:* str

Key of the object.

---

##### `tryGet` <a name="tryGet" id="@winglang/sdk.cloud.IBucketClient.tryGet"></a>

```wing
inflight tryGet(key: str): str
```

Get an object from the bucket if it exists.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.tryGet.parameter.key"></a>

- *Type:* str

Key of the object.

---

##### `tryGetJson` <a name="tryGetJson" id="@winglang/sdk.cloud.IBucketClient.tryGetJson"></a>

```wing
inflight tryGetJson(key: str): Json
```

Gets an object from the bucket if it exists, parsing it as Json.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketClient.tryGetJson.parameter.key"></a>

- *Type:* str

Key of the object.

---


#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.Bucket.property.node">node</a></code> | <code>constructs.Node</code> | The tree node. |
| <code><a href="#@winglang/sdk.cloud.Bucket.property.display">display</a></code> | <code><a href="#@winglang/sdk.std.Display">Display</a></code> | Information on how to display a resource in the UI. |

---

##### `node`<sup>Required</sup> <a name="node" id="@winglang/sdk.cloud.Bucket.property.node"></a>

```wing
node: Node;
```

- *Type:* constructs.Node

The tree node.

---

##### `display`<sup>Required</sup> <a name="display" id="@winglang/sdk.cloud.Bucket.property.display"></a>

```wing
display: Display;
```

- *Type:* <a href="#@winglang/sdk.std.Display">Display</a>

Information on how to display a resource in the UI.

---



## Structs <a name="Structs" id="Structs"></a>

### BucketDeleteOptions <a name="BucketDeleteOptions" id="@winglang/sdk.cloud.BucketDeleteOptions"></a>

Interface for delete method inside `Bucket`.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketDeleteOptions.Initializer"></a>

```wing
bring cloud;

let BucketDeleteOptions = cloud.BucketDeleteOptions{ ... };
```

#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.BucketDeleteOptions.property.mustExist">mustExist</a></code> | <code>bool</code> | Check failures on the method and retrieve errors if any. |

---

##### `mustExist`<sup>Optional</sup> <a name="mustExist" id="@winglang/sdk.cloud.BucketDeleteOptions.property.mustExist"></a>

```wing
mustExist: bool;
```

- *Type:* bool
- *Default:* false

Check failures on the method and retrieve errors if any.

---

### BucketEvent <a name="BucketEvent" id="@winglang/sdk.cloud.BucketEvent"></a>

on_event notification payload- will be in use after solving issue: https://github.com/winglang/wing/issues/1927.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketEvent.Initializer"></a>

```wing
bring cloud;

let BucketEvent = cloud.BucketEvent{ ... };
```

#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.BucketEvent.property.key">key</a></code> | <code>str</code> | the bucket key that triggered the event. |
| <code><a href="#@winglang/sdk.cloud.BucketEvent.property.type">type</a></code> | <code><a href="#@winglang/sdk.cloud.BucketEventType">BucketEventType</a></code> | type of event. |

---

##### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.BucketEvent.property.key"></a>

```wing
key: str;
```

- *Type:* str

the bucket key that triggered the event.

---

##### `type`<sup>Required</sup> <a name="type" id="@winglang/sdk.cloud.BucketEvent.property.type"></a>

```wing
type: BucketEventType;
```

- *Type:* <a href="#@winglang/sdk.cloud.BucketEventType">BucketEventType</a>

type of event.

---

### BucketOnCreateProps <a name="BucketOnCreateProps" id="@winglang/sdk.cloud.BucketOnCreateProps"></a>

on create event options.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketOnCreateProps.Initializer"></a>

```wing
bring cloud;

let BucketOnCreateProps = cloud.BucketOnCreateProps{ ... };
```


### BucketOnDeleteProps <a name="BucketOnDeleteProps" id="@winglang/sdk.cloud.BucketOnDeleteProps"></a>

on delete event options.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketOnDeleteProps.Initializer"></a>

```wing
bring cloud;

let BucketOnDeleteProps = cloud.BucketOnDeleteProps{ ... };
```


### BucketOnEventProps <a name="BucketOnEventProps" id="@winglang/sdk.cloud.BucketOnEventProps"></a>

on any event options.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketOnEventProps.Initializer"></a>

```wing
bring cloud;

let BucketOnEventProps = cloud.BucketOnEventProps{ ... };
```


### BucketOnUpdateProps <a name="BucketOnUpdateProps" id="@winglang/sdk.cloud.BucketOnUpdateProps"></a>

on update event options.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketOnUpdateProps.Initializer"></a>

```wing
bring cloud;

let BucketOnUpdateProps = cloud.BucketOnUpdateProps{ ... };
```


### BucketProps <a name="BucketProps" id="@winglang/sdk.cloud.BucketProps"></a>

Properties for `Bucket`.

#### Initializer <a name="Initializer" id="@winglang/sdk.cloud.BucketProps.Initializer"></a>

```wing
bring cloud;

let BucketProps = cloud.BucketProps{ ... };
```

#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.BucketProps.property.public">public</a></code> | <code>bool</code> | Whether the bucket's objects should be publicly accessible. |

---

##### `public`<sup>Optional</sup> <a name="public" id="@winglang/sdk.cloud.BucketProps.property.public"></a>

```wing
public: bool;
```

- *Type:* bool
- *Default:* false

Whether the bucket's objects should be publicly accessible.

---

## Protocols <a name="Protocols" id="Protocols"></a>

### IBucketEventHandler <a name="IBucketEventHandler" id="@winglang/sdk.cloud.IBucketEventHandler"></a>

- *Extends:* <a href="#@winglang/sdk.std.IResource">IResource</a>

- *Implemented By:* <a href="#@winglang/sdk.cloud.IBucketEventHandler">IBucketEventHandler</a>

**Inflight client:** [@winglang/sdk.cloud.IBucketEventHandlerClient](#@winglang/sdk.cloud.IBucketEventHandlerClient)

A resource with an inflight "handle" method that can be passed to the bucket events.


#### Properties <a name="Properties" id="Properties"></a>

| **Name** | **Type** | **Description** |
| --- | --- | --- |
| <code><a href="#@winglang/sdk.cloud.IBucketEventHandler.property.node">node</a></code> | <code>constructs.Node</code> | The tree node. |
| <code><a href="#@winglang/sdk.cloud.IBucketEventHandler.property.display">display</a></code> | <code><a href="#@winglang/sdk.std.Display">Display</a></code> | Information on how to display a resource in the UI. |

---

##### `node`<sup>Required</sup> <a name="node" id="@winglang/sdk.cloud.IBucketEventHandler.property.node"></a>

```wing
node: Node;
```

- *Type:* constructs.Node

The tree node.

---

##### `display`<sup>Required</sup> <a name="display" id="@winglang/sdk.cloud.IBucketEventHandler.property.display"></a>

```wing
display: Display;
```

- *Type:* <a href="#@winglang/sdk.std.Display">Display</a>

Information on how to display a resource in the UI.

---

### IBucketEventHandlerClient <a name="IBucketEventHandlerClient" id="@winglang/sdk.cloud.IBucketEventHandlerClient"></a>

- *Implemented By:* <a href="#@winglang/sdk.cloud.IBucketEventHandlerClient">IBucketEventHandlerClient</a>

A resource with an inflight "handle" method that can be passed to the bucket events.

#### Methods <a name="Methods" id="Methods"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.cloud.IBucketEventHandlerClient.handle">handle</a></code> | Function that will be called when an event notification is fired. |

---

##### `handle` <a name="handle" id="@winglang/sdk.cloud.IBucketEventHandlerClient.handle"></a>

```wing
inflight handle(key: str, type: BucketEventType): void
```

Function that will be called when an event notification is fired.

###### `key`<sup>Required</sup> <a name="key" id="@winglang/sdk.cloud.IBucketEventHandlerClient.handle.parameter.key"></a>

- *Type:* str

---

###### `type`<sup>Required</sup> <a name="type" id="@winglang/sdk.cloud.IBucketEventHandlerClient.handle.parameter.type"></a>

- *Type:* <a href="#@winglang/sdk.cloud.BucketEventType">BucketEventType</a>

---


## Enums <a name="Enums" id="Enums"></a>

### BucketEventType <a name="BucketEventType" id="@winglang/sdk.cloud.BucketEventType"></a>

bucket events to subscribe to.

#### Members <a name="Members" id="Members"></a>

| **Name** | **Description** |
| --- | --- |
| <code><a href="#@winglang/sdk.cloud.BucketEventType.CREATE">CREATE</a></code> | create. |
| <code><a href="#@winglang/sdk.cloud.BucketEventType.DELETE">DELETE</a></code> | delete. |
| <code><a href="#@winglang/sdk.cloud.BucketEventType.UPDATE">UPDATE</a></code> | update. |

---

##### `CREATE` <a name="CREATE" id="@winglang/sdk.cloud.BucketEventType.CREATE"></a>

create.

---


##### `DELETE` <a name="DELETE" id="@winglang/sdk.cloud.BucketEventType.DELETE"></a>

delete.

---


##### `UPDATE` <a name="UPDATE" id="@winglang/sdk.cloud.BucketEventType.UPDATE"></a>

update.

---

