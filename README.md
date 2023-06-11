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

This project is an implementation of the Kamilata protocol. It takes the form of a library that you can easily plug into your own platform. This library provides a simple yet very powerful API to index any document you might have.

This library is intented for various use cases, such as:

- Youtube-like video sharing platforms
- Social networks
- File sharing platforms
- Web search engines

Kamilata is relevant and able to provide a good search experience if *one* of the following conditions is met:

- Queries are specific (a few words)
- The corpus is small (less than a million documents)
- There are significant popularity differences between documents

Kamilata powers the [Admarus IPFS search engine](https://github.com/mubelotix/admarus-daemon).

## General Technical Description

The Kamilata routing algorithm is based on [Attenuated Bloom Filters](https://en.wikipedia.org/wiki/Bloom_filter#Attenuated_Bloom_filters). Bloom filters are compact data structures used to determine if an element is present in a set. Here, we check the presence of words in documents. From a node's point of view, a Kamilata network is divided into virtual node groups of varying sizes. This divides the corpus into multiple sets ranging from a few documents to all documents of the corpus. Each having its corresponding Bloom filter, it is then easy to locate words in the network and know which nodes to query for given words.
