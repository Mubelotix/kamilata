<p align="center">
  <img src="https://raw.githubusercontent.com/Mubelotix/kamilata/master/.github/logo.png" alt="Kamilata" width="200" height="200" />
</p>

<h1 align="center">Kamilata</h1>

<p align="center">
    <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
    <img alt="Lines of code" src="https://img.shields.io/tokei/lines/github/Mubelotix/kamilata">
    <img alt="GitHub last commit" src="https://img.shields.io/github/last-commit/Mubelotix/kamilata?color=%23347d39" alt="last commit badge">
    <a href="https://wakatime.com/badge/user/6a4c28c6-c833-460a-815e-15ce48b15c25/project/0e6a2208-db21-4763-aff7-35f4abc1773f"><img src="https://wakatime.com/badge/user/6a4c28c6-c833-460a-815e-15ce48b15c25/project/0e6a2208-db21-4763-aff7-35f4abc1773f.svg" alt="wakatime"></a>
    <img alt="GitHub closed issues" src="https://img.shields.io/github/issues-closed-raw/Mubelotix/kamilata?color=%23347d39" alt="closed issues badge">
</p>

<p align="center">A Peer-to-Peer Search Engine System</p>

## Scope

This project is an implementation as a library of the Kamilata protocol.
Kamilata enables trustless search in open networks.
This library can handle any type of data, and be easily integrated into your libp2p application.

Several use cases are possible:

- Youtube-like video sharing platforms
- Social networks
- File sharing platforms
- Web search engines

The ranking algorithm is up to you, as this library will only provide you a stream of unordered search results.
Based on metadata you include in those results, you can rank them however you want.

Kamilata is the first system in the world to offer the properties described above, while still being scalable.
Indeed, the network can include without problems more than hundreds of millions documents and hundreds of thousands of nodes.
The actual limit is unknown.

<!-- Kamilata starts being irrelevant for a query when the first 10 most relevant results for that query are not provided by more than 0.1% of the peers who have matching documents for that query. This is completely impossible in networks of less than 1000 peers. In other cases, it's still very uncommon, especially if the most relevant results are also the most popular ones.  TODO clarify -->

This library powers the [Admarus IPFS search engine](https://github.com/mubelotix/admarus).

## General Technical Description

The Kamilata routing algorithm is based on [Attenuated Bloom Filters](https://en.wikipedia.org/wiki/Bloom_filter#Attenuated_Bloom_filters). Bloom filters are compact data structures used to determine if an element is present in a set. Here, we check the presence of words in documents. From a node's point of view, a Kamilata network is divided into virtual node groups of varying sizes. This divides the corpus into multiple sets ranging from a few documents to all documents of the corpus. Each having its corresponding Bloom filter, it is then easy to locate words in the network and know which nodes to query for given words.
