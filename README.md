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


## Abstract

Search engines have always been quintessentially centralized systems. The need for a central database to store and index gigantic amounts of data has consecrated big companies as the only ones able to provide such a service. After years of accumulating power and influence, these same companies have started abusing their position, manipulating search results, censoring content, and spying on their users. As those in control of searches rule which content is reachable, they have become the new gatekeepers of the Internet.

A purely peer-to-peer version of a search engine would allow the search of data without the need of relying on any authority. The network formed by users would be directly in charge of the content, with no intermediaries. This is Kamilata. It features a routing algorithm for redirecting search queries to the peers that are most likely to have matching results. Thanks to this approach, no central index is required. As a result, peers can join and leave the network freely, without any coordination needed at the network level. 

## General Technical Description

The Kamilata routing algorithm is based on [Attenuated Bloom Filters](https://en.wikipedia.org/wiki/Bloom_filter#Attenuated_Bloom_filters). Bloom filters are compact data structures used to determine if an element is present in a set. Here, we check the presence of words in documents. From a node's point of view, a Kamilata network is divided into virtual node groups of varying sizes. This divides the corpus into multiple sets ranging from a few documents to all documents of the corpus. Each having its corresponding Bloom filter, it is then easy to locate words in the network and know which nodes to query for given words.
