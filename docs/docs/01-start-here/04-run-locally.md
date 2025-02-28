---
title: Run locally
id: local
keywords: [Wing Local, Wing testing, Wing Console installation, Wing Console usage, Wing Console setup]
---

The **Wing Console** is a web application that provides an interface that can be used to view, explore, and interact with Wing applications as you develop them on the local machine.

## Starting the Wing Console

Use the `wing it` command to start the console with our newly created application:

```sh
wing it hello.w
```

This will now compile `hello.w` to [the "`sim`" target](../concepts/simulator),
and load the application in the **Wing Console**.

In the Wing Console window, you will see the following view of you application:

![Wing Console desktop application view](./console-app.png 'Wing Console')

> **You are on auto pilot**: Wing Console will "hot reload" your app on each change to 
> the source file from now on. Every time you save your code, a fresh copy of your
> app will be reloaded in the console.

## The Map View

In the console [map view](../06-tools/02-wing-console.md#map-view) you'll see three resources: a **Queue**, a **Function** and a **Bucket**. The queue and the bucket are connected through the function resource, which serves as a message handler and is set as the consumer for the queue.

Click on the queue resource and check out to the right-hand panel. This is the [resource interaction panel](../tools/wing-console#interact-with-your-wing-application).

![Queue resource view in Wing Console](./console-queue.png 'Queue resource')

## Push a message to the queue

On the interaction panel, Type a message in the `Push Message` text area (let's say `Wing it`) and hit **Push** in order to push it to the queue.

## View the file in the bucket

Now, click on the bucket in the map view. Notice that the interaction panel has changed and now shows the bucket's interaction view.

When you click on the `wing.txt` file, you should be able to see its contents.

![Bucket resource view in Wing Console](./console-bucket-1.png 'Bucket resource')

You can also download the file, using the **Download** button.

The downloaded file should contain `Hello, Wing it` text (as you already saw in the preview).

## Congrats! :clap:

You have just written and ran your first Wing program!

---

Now, after you have made sure your application works, lets deploy it to AWS.
