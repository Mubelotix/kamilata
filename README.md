# Kamilata: A Peer-to-Peer Search Engine System

## Abstract

Search engines have always been quintessentially centralized systems. The need for a central database to store and index gigantic amounts of data has consecrated big companies as the only ones able to provide such a service. After years of accumulating power and influence, these same companies have started abusing their position, manipulating search results, censoring content, and spying on their users. As those in control of searches rule which content is reachable, they have become the new gatekeepers of the Internet.

A purely peer-to-peer version of a search engine would allow the search of data without the need of relying on any authority. The network formed by users would be directly in charge of the content, with no intermediaries. This is Kamilata. It features a routing algorithm for redirecting search queries to the peers that are most likely to have matching results. Thanks to this approach, no central index is required. As a result, peers can join and leave the network freely, without any coordination needed at the network level. 

## General Technical Description

The Kamilata routing algorithm is based on [Attenuated Bloom Filters](https://en.wikipedia.org/wiki/Bloom_filter#Attenuated_Bloom_filters). Bloom filters are compact data structures used to determine if an element is present in a set. Here, we check the presence of words in documents. From a node's point of view, a Kamilata network is divided into virtual node groups of varying sizes. This divides the corpus into multiple sets ranging from a few documents to all documents of the corpus. Each having its corresponding Bloom filter, it is then easy to locate words in the network and know which nodes to query for given words.
